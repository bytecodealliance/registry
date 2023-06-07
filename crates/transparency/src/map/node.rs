use alloc::sync::Arc;
use alloc::vec::Vec;
use std::fmt::Debug;

use warg_crypto::hash::{Hash, SupportedDigest};
use warg_crypto::VisitBytes;

use super::fork::Fork;
use super::link::Link;
use super::path::{Path, Side};
use super::proof::Proof;
use super::singleton::Singleton;

#[derive(Debug)]
pub enum Node<D: SupportedDigest, K: Debug + VisitBytes + Clone + PartialEq> {
    Leaf(Hash<D>),
    Fork(Fork<D, K>),
    Singleton(Singleton<D, K>),
    Empty(usize),
}

impl<D: SupportedDigest, K: Debug + VisitBytes + Clone + PartialEq> Clone for Node<D, K> {
    fn clone(&self) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.clone()),
            Self::Fork(node) => Self::Fork(node.clone()),
            Self::Singleton(s) => Self::Singleton(s.clone()),
            Self::Empty(height) => Self::Empty(*height),
        }
    }
}

impl<D: SupportedDigest, K: Debug + VisitBytes + Clone + PartialEq> Default for Node<D, K> {
    fn default() -> Self {
        Self::Fork(Fork::new(
            Arc::new(Link::new(Node::Empty(0))),
            Arc::new(Link::new(Node::Empty(0))),
        ))
    }
}

impl<D: SupportedDigest, K: Debug + VisitBytes + Clone + PartialEq> Node<D, K> {
    pub fn hash(&self) -> Hash<D> {
        match self {
            Node::Leaf(hash) => hash.clone(),
            Node::Fork(fork) => fork.hash(),
            Node::Singleton(singleton) => singleton.hash(),
            Node::Empty(height) => D::empty_tree_hash(*height),
        }
    }

    pub fn prove<V: VisitBytes + Clone>(&self, mut path: Path<D, K>) -> Option<Proof<D, K, V>> {
        match (path.next(), self) {
            (Some(_), Self::Singleton(singleton)) => {
                if singleton.key() == path.key() {
                    Some(Proof::new(Vec::new()))
                } else {
                    None
                }
            }
            (Some(_), Self::Empty(_)) => {
                let mut proof: Proof<D, K, V> = self.prove(path)?;
                proof.push(None);
                None
            }
            (Some(idx), Self::Fork(fork)) => {
                let proof = fork[idx].as_ref().node().prove(path);
                match proof {
                  Some(mut p) => {
                    let peer = fork[idx.opposite()].as_ref().hash();
                    p.push(Some(peer.clone()));
                    Some(p)

                  }
                  None => None
                }
            }

            (None, Self::Leaf(_)) => Some(Proof::new(Vec::new())),

            _ => None,
        }
    }

    /// A recursive function for setting the value in the tree.
    ///
    /// Returns:
    ///   * the new node that must replace the current node.
    ///   * whether or not this is a new entry in the map.
    pub fn insert(&self, path: &mut Path<D, K>, key: K, value: Hash<D>) -> (Self, bool) {
        match path.next() {
            // We are at the end of the path. Save the leaf.
            None => (
                Node::Leaf(value),
                matches!(self, Node::Empty(_)) || matches!(self, Node::Singleton(_)),
            ),

            // We are not at the end of the path. Recurse...
            Some(index) => match self.clone() {
                Node::Empty(_) => {
                    let singleton =
                        Node::Singleton(Singleton::new(key, value, 256 - path.index(), index));
                    if path.index() == 1 {
                        match index {
                            Side::Left => {
                                let fork = Fork::new(
                                    Arc::new(Link::new(singleton)),
                                    Arc::new(Link::new(Node::Empty(256 - path.index()))),
                                );
                                return (Node::Fork(fork), true);
                            }
                            Side::Right => {
                                let fork = Fork::new(
                                    Arc::new(Link::new(Node::Empty(256 - path.index()))),
                                    Arc::new(Link::new(singleton)),
                                );
                                return (Node::Fork(fork), true);
                            }
                        }
                    }
                    (singleton, true)
                }
                Node::Fork(mut fork) => {
                    // Choose the branch on the specified side.
                    let node = fork[index].as_ref().node();
                    // fork
                    match node {
                        Node::Empty(_) => {
                            let singleton = Node::Singleton(Singleton::new(
                                key,
                                value,
                                256 - path.index(),
                                index,
                            ));
                            fork[index] = Arc::new(Link::new(singleton));
                            (Node::Fork(fork), true)
                        }
                        Node::Singleton(singleton) => {
                            if singleton.key() == key {
                                let new_singleton = Node::Singleton(Singleton::new(
                                    key,
                                    value,
                                    256 - path.index(),
                                    index,
                                ));
                                fork[index] = Arc::new(Link::new(new_singleton));
                                (Node::Fork(fork), false)
                            } else {
                                let (new_fork, new) = node.insert(path, key, value);
                                let new_node = Arc::new(Link::new(new_fork));
                                fork[index] = new_node;
                                (Node::Fork(fork), new)
                            }
                        }
                        _ => {
                            // Replace its value recursively.
                            let (node, new) = node.insert(path, key, value);
                            fork[index] = Arc::new(Link::new(node));
                            (Node::Fork(fork), new)
                        }
                    }
                }
                Node::Singleton(singleton) => {
                    if singleton.key() == key {
                        let new_singleton = Singleton::new(key, value, 256 - path.index(), index);
                        (Node::Singleton(new_singleton), false)
                    } else if singleton.side != index {
                        let node =
                            Node::Singleton(Singleton::new(key, value, 256 - path.index(), index));
                        let original = Node::Singleton(Singleton::new(
                            singleton.key,
                            singleton.value,
                            256 - path.index(),
                            index.opposite(),
                        ));
                        let fork = match index {
                            Side::Left => {
                                Fork::new(Arc::new(Link::new(node)), Arc::new(Link::new(original)))
                            }
                            Side::Right => {
                                Fork::new(Arc::new(Link::new(original)), Arc::new(Link::new(node)))
                            }
                        };
                        (Node::Fork(fork), false)
                    } else {
                        let fork = match index {
                            Side::Left => {
                                let (down_one, _) = Node::Singleton(Singleton::new(
                                    singleton.key,
                                    singleton.value,
                                    singleton.height - 1,
                                    index,
                                ))
                                .insert(path, key, value);
                                Fork::new(
                                    Arc::new(Link::new(down_one)),
                                    Arc::new(Link::new(Node::Empty(256 - path.index() - 1))),
                                )
                            }
                            Side::Right => {
                                let (down_one, _) = Node::Singleton(Singleton::new(
                                    singleton.key,
                                    singleton.value,
                                    singleton.height - 1,
                                    index,
                                ))
                                .insert(path, key, value);
                                Fork::new(
                                    Arc::new(Link::new(Node::Empty(256 - path.index() - 1))),
                                    Arc::new(Link::new(down_one)),
                                )
                            }
                        };
                        (Node::Fork(fork), true)
                    }
                }
                Node::Leaf(_) => (Node::Leaf(value), false),
            },
        }
    }
}
