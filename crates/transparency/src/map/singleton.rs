use std::{error::Error, fmt, sync::Arc, fs::{File, self}, io::Write};

use warg_crypto::hash::{Hash, SupportedDigest};

use crate::map::{fork::Fork, link::Link};

use super::{
    map::hash_branch,
    node::Node,
    path::{Path, ReversePath, Side},
};

#[derive(Debug)]
pub struct Singleton<D: SupportedDigest> {
    pub key: Hash<D>,
    pub value: Hash<D>,
    pub elided_value: Hash<D>,
}

#[derive(Debug)]
pub struct ReplacementError {
    details: String,
}

impl ReplacementError {
    pub fn new() -> Self {
        Self {
            details: String::from("Elided path length not equal to inerted path length"),
        }
    }
}

impl fmt::Display for ReplacementError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for ReplacementError {
    fn description(&self) -> &str {
        &self.details
    }
}

impl<D: SupportedDigest + std::fmt::Debug> Singleton<D> {
    pub fn new(
        key: Hash<D>,
        value: Hash<D>,
        path: &Path<D>,
        reversed: &mut ReversePath<D>,
    ) -> Self {
        // dbg!("NEW SINGLETON");
        // dbg!(key.clone());
        let cur = 256 - path.index();
        let elided_value = value.clone();
        let mut hash = value;
        for n in 0..cur {
            hash = match reversed.next() {
                Some(side) => match side {
                    Side::Left => {
                      // dbg!(side);
                      hash_branch(Some(hash.clone()), Some(D::empty_tree_hash(n)))
                    },
                    Side::Right => {
                      // dbg!(side);
                      hash_branch(Some(D::empty_tree_hash(n)), Some(hash.clone()))
                    },
                },
                None => hash.clone(),
            };
            dbg!(&hash);
        }
        Self {
            key,
            value: hash,
            elided_value,
        }
    }

    pub fn replace(
        &self,
        key: Hash<D>,
        elided_value: Hash<D>,
        path: &mut Path<D>,
        reversed: &mut ReversePath<D>,
        new_key: Hash<D>,
        new_val: Hash<D>,
    ) -> Node<D> {
        let orig_index = path.index();
        dbg!(orig_index);
        let mut elided_path = Path::new(key.clone());
        while elided_path.index() != path.index() {
            elided_path.next();
        }
        let mut finished = false;
        let mut fused_point = orig_index;
        let mut fused_side = Side::Left;
        while !finished {
            let next_index = path.next();
            let next_elided_index = elided_path.next();
            if let Some(index) = next_index {
                if let Some(elided_index) = next_elided_index {
                    if index == elided_index {
                        fused_point += 1;
                    } else {
                      fused_side = index;
                        finished = true;
                        dbg!("different boys");
                    }
                }
            }
        }

        dbg!(fused_point);
        let mut elided_reversed = ReversePath::new(key.clone());
        // reversed.back();
        // elided_reversed.back();
        let singleton = Singleton::new(new_key, new_val, path, reversed);
        let elided_singleton = Singleton::new(key, elided_value, &elided_path, &mut elided_reversed);

        dbg!(singleton.clone());
        dbg!(elided_singleton.clone());
        dbg!(elided_singleton.hash());

        let mut fused = Node::Empty(0);
        // if let Some(side) = reversed.next(){
            match fused_side {
                Side::Left => {
                    fused = Node::Fork(Fork::new(
                      Arc::new(Link::new(Node::Singleton(singleton))),
                      Arc::new(Link::new(Node::Singleton(elided_singleton))),
                    ));
                    dbg!(fused.hash());
                }
                Side::Right => {
                    fused = Node::Fork(Fork::new(
                      Arc::new(Link::new(Node::Singleton(elided_singleton))),
                      Arc::new(Link::new(Node::Singleton(singleton))),
                    ));
                    dbg!(fused.hash());
                }
            }
        // }
        dbg!(&fused);
        dbg!(reversed.index(), orig_index, fused_point);
        for n in orig_index..reversed.index() - 1 {
          dbg!(n);
            if let Some(side) = reversed.next() {
                dbg!(fused.hash());
                match side {
                    Side::Left => {
                        let temp = fused.clone();
                        // dbg!(temp.hash());
                        fused = Node::Fork(Fork::new(
                            Arc::new(Link::new(temp.clone())),
                            Arc::new(Link::new(Node::Empty(256 - reversed.index()))),
                        ));
                    }
                    Side::Right => {
                        let temp = fused.clone();
                        fused = Node::Fork(Fork::new(
                            Arc::new(Link::new(temp)),
                            Arc::new(Link::new(Node::Empty(256 - reversed.index()))),
                        ));
                    }
                }
                dbg!(fused.hash());
            }
        }
        fused

    }

    pub fn key(&self) -> Hash<D> {
        self.key.clone()
    }

    pub fn hash(&self) -> Hash<D> {
        self.value.clone()
    }

    pub fn elided_hash(&self) -> Hash<D> {
        self.elided_value.clone()
    }
}

impl<D: SupportedDigest> Clone for Singleton<D> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            value: self.value.clone(),
            elided_value: self.elided_value.clone(),
        }
    }
}

//             let singleton =
//                 Singleton::new(new_key.clone(), new_val.clone(), path, reversed);
//             let elided_singleton = Singleton::new(
//                 key.clone(),
//                 value.clone(),
//                 &elided_path,
//                 &mut reversed_elided_path,
//             );
//             // dbg!(&singleton, &elided_singleton);
//             match index {
//                 Side::Left => {
//                     let fork = Fork::new(
//                         Arc::new(Link::new(Node::Singleton(singleton))),
//                         Arc::new(Link::new(Node::Singleton(elided_singleton))),
//                     );
//                     Ok(fork)
//                 }
//                 Side::Right => {
//                     let fork = Fork::new(
//                         Arc::new(Link::new(Node::Singleton(elided_singleton))),
//                         Arc::new(Link::new(Node::Singleton(singleton))),
//                     );
//                     dbg!(reversed.index());
//                     Ok(fork)
//                 }
//             }
//         }
//         None => Err(ReplacementError::new()),
//     }
// }
// None => {
//     let fork = match next_elided_index {
//         Some(elided_index) => Err(ReplacementError::new()),
//         None => Ok({
//             Fork::new(
//                 Arc::new(Link::new(Node::Empty(0))),
//                 Arc::new(Link::new(Node::Empty(0))),
//             )
//         }),
//     };
//     fork
// }
// }
