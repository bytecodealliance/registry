use alloc::vec::Vec;
use warg_crypto::hash::{Hash, SupportedDigest};

use super::{hash_branch, hash_leaf, node::Node, Checkpoint, LogBuilder, hash_empty};

/// A log builder which maintains a stack of balanced roots
#[derive(Clone, Debug, Default)]
pub struct StackLog<D>
where
    D: SupportedDigest,
{
    stack: Vec<(Node, Hash<D>)>,
    length: usize,
}

impl<D> LogBuilder<D> for StackLog<D>
where
    D: SupportedDigest,
{
    fn checkpoint(&self) -> Checkpoint<D> {
        let root = self
            .stack
            .iter()
            .rev()
            .map(|(_n, hash)| hash.clone())
            .reduce(|new, old| {
                hash_branch::<D>(old, new)
            })
            .unwrap_or(hash_empty::<D>());

        Checkpoint {
            root,
            length: self.length,
        }
    }

    fn push(&mut self, entry: impl AsRef<[u8]>) -> Node {
        let node = Node(self.length * 2);

        let leaf_digest = hash_leaf::<D>(entry);

        self.length += 1;
        self.stack.push((node, leaf_digest));
        self.reduce();

        node
    }
}

impl<D> StackLog<D>
where
    D: SupportedDigest,
{
    fn reduce(&mut self) {
        while self.stack.len() >= 2 {
            let (top_node, top_hash) = &self.stack[self.stack.len() - 1];
            let (second_node, second_hash) = &self.stack[self.stack.len() - 2];

            if top_node.height() == second_node.height() {
                let new_node = top_node.parent();
                let new_hash = hash_branch::<D>(second_hash.clone(), top_hash.clone());
                self.stack.pop();
                self.stack.pop();
                self.stack.push((new_node, new_hash));
            } else {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use warg_crypto::hash::Sha256;

    use super::*;
    use super::super::VecLog;

    #[test]
    fn test_matches_vec_log() {
        let mut vec_log: VecLog<Sha256> = VecLog::default();
        let mut stack_log: StackLog<Sha256> = StackLog::default();

        let data: [&str; 25] = [
            "93", "67", "30", "37", "23", "75", "57", "89", "76", "42", "9", "14", "40", "59",
            "26", "66", "77", "38", "47", "34", "8", "81", "101", "102", "103",
        ];

        for leaf in data {
            vec_log.push(leaf);
            stack_log.push(leaf);

            assert_eq!(vec_log.checkpoint(), stack_log.checkpoint());
        }
    }
}