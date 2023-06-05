use alloc::sync::Arc;
use alloc::vec::Vec;

// use sha2::Sha256;
use warg_crypto::hash::{Hash, SupportedDigest};
use warg_crypto::VisitBytes;

use super::fork::Fork;
use super::link::Link;
use super::path::{Path, ReversePath, Side};
use super::proof::Proof;
use super::singleton::Singleton;

#[derive(Debug)]
pub enum Node<D: SupportedDigest + std::fmt::Debug> {
    Leaf(Hash<D>),
    Fork(Fork<D>),
    Singleton(Singleton<D>),
    Empty(usize),
}

impl<D: SupportedDigest + std::fmt::Debug> Clone for Node<D> {
    fn clone(&self) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.clone()),
            Self::Fork(node) => Self::Fork(node.clone()),
            Self::Singleton(s) => Self::Singleton(s.clone()),
            Self::Empty(height) => Self::Empty(*height),
        }
    }
}

impl<D: SupportedDigest + std::fmt::Debug> Default for Node<D> {
    fn default() -> Self {
        Self::Fork(Fork::new(
            Arc::new(Link::new(Node::Empty(0))),
            Arc::new(Link::new(Node::Empty(0))),
        ))
    }
}

impl<D: SupportedDigest + std::fmt::Debug> Node<D> {
    pub fn hash(&self) -> Hash<D> {
        match self {
            Node::Leaf(hash) => hash.clone(),
            Node::Fork(fork) => fork.hash(),
            Node::Singleton(singleton) => singleton.hash(),
            Node::Empty(height) => D::empty_tree_hash(*height),
        }
    }

    pub fn prove<V: VisitBytes>(&self, mut path: Path<D>, potential_peer: Option<Node<D>>) -> Option<Proof<D, V>> {
        // dbg!("proving");
        // dbg!(&self.);
        match (path.next(), self) {
            (Some(_), Self::Singleton(singleton)) => {
                dbg!("SINGLETON");
                dbg!(path.hash(), singleton.key());
                if singleton.key() == path.hash() {
                  dbg!("MATCHEd");
                  // if let Some(peer) = potential_peer {
                  //   let mut proof = Vec::new();
                  //   proof.push(Some(peer.hash()));
                  //   Some(Proof::new(proof))
                  // } else {
                    Some(Proof::new(Vec::new()))
                  // }
                } else {
                  // match potential_peer {
                  //   Node_Singleton(_) => {

                  //   }
                  // }
                    Some(Proof::new(Vec::new()))
                    // None
                }
            }
            (Some(_), Self::Empty(_)) => {
                dbg!("EMPTY PROOF");
                let mut proof: Proof<D, V> = self.prove(path, None)?;
                proof.push(None);
                None
            }
            (Some(idx), Self::Fork(fork)) => {
                dbg!("FORK PROOF");
                dbg!(&fork);
                dbg!(path.index());
                // dbg!(fork[idx].as_ref().node().hash());
                // match fork[idx.opposite()].as_ref().node() {
                //   Node::Empty(_) => {

                //   }
                //   Node::Fork(_) => {
                //     dbg!("FORK");
                //   }
                //   _ => {}
                // }
                dbg!(fork[idx.opposite()].as_ref().node());
                let potential_peer = fork[idx.opposite()].as_ref().node();
                let mut proof = fork[idx].as_ref().node().prove(path, Some(potential_peer.clone()))?;
                
                let peer = fork[idx.opposite()].as_ref().hash();
                dbg!(peer.clone());
                proof.push(Some(peer.clone()));
                dbg!(proof.peers.clone());
                Some(proof)
            }

            (None, Self::Leaf(_)) => {
              // dbg!("NONE PROOF");
              Some(Proof::new(Vec::new()))
            },

            _ => {
              // dbg!("OTHER");
               None
            },
        }
    }

    /// A recursive function for setting the value in the tree.
    ///
    /// Returns:
    ///   * the new node that must replace the current node.
    ///   * whether or not this is a new entry in the map.
    pub fn insert(
        &self,
        path: &mut Path<D>,
        reversed: &mut ReversePath<D>,
        key: Hash<D>,
        value: Hash<D>,
    ) -> (Self, bool) {
      dbg!("INSERT KEY", key.clone());
      dbg!(value.clone());
        match path.next() {
            // We are at the end of the path. Save the leaf.
            None => (
                Node::Leaf(value),
                matches!(self, Node::Empty(_)) || matches!(self, Node::Singleton(_)),
            ),

            // We are not at the end of the path. Recurse...
            Some(index) => match self.clone() {
                Node::Empty(_) => {
                    let singleton = Node::Singleton(Singleton::new(key, value, path, reversed));
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
                    dbg!(&fork, index);
                    // Choose the branch on the specified side.
                    let node = fork[index].as_ref().node();
                    // fork
                    match node {
                        Node::Empty(_) => {
                            let singleton =
                                Node::Singleton(Singleton::new(key, value, path, reversed));
                            fork[index] = Arc::new(Link::new(singleton));
                            (Node::Fork(fork), true)
                        }
                        Node::Singleton(singleton) => {
                            if singleton.key() == key {
                                let new_singleton =
                                    Node::Singleton(Singleton::new(key, value, path, reversed));
                                fork[index] = Arc::new(Link::new(new_singleton));
                                (Node::Fork(fork), false)
                            } else {
                                dbg!("ABOUT TO REPLACE");
                                let (new_fork, new) = node.insert(path, reversed, key, value);
                                let new_node = Arc::new(Link::new(new_fork));
                                // let new_node = Arc::new(Link::new(Node::Fork(Fork::new(
                                //   Arc::new(Link::new(new_fork)),
                                //   Arc::new(Link::new(fork[index.opposite()].node().clone()))
                                // ))));
                                fork[index] = new_node;
                                (Node::Fork(fork), new)
                            }
                        }
                        _ => {
                            // Replace its value recursively.
                            let (node, new) = node.insert(path, reversed, key, value);
                            fork[index] = Arc::new(Link::new(node));
                            (Node::Fork(fork), new)
                        }
                    }
                }
                Node::Singleton(singleton) => {
                    if singleton.key() == key {
                        let new_singleton = Singleton::new(key, value, path, reversed);
                        (Node::Singleton(new_singleton), false)
                    } else {
                        // let node = Node::Singleton(Singleton::new(key.clone(), value.clone(), path, reversed));
                        let node = singleton.replace(
                            singleton.key(),
                            singleton.elided_hash(),
                            path,
                            reversed,
                            key,
                            value,
                        );
                        // dbg!(node.unwrap())
                        let foo = node;
                        dbg!(&foo);
                        (foo, true)
                    }
                }
                Node::Leaf(_) => (Node::Leaf(value), false),
            },
        }
    }
}
