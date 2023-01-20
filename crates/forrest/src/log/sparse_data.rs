use alloc::vec::Vec;
use warg_crypto::hash::{Hash, SupportedDigest};

use super::{LogData, Node};

pub struct SparseLogData<D>
where
    D: SupportedDigest,
{
    pub(crate) data: Vec<(Node, Hash<D>)>,
}

impl<D> SparseLogData<D>
where
    D: SupportedDigest,
{
    fn get_index(&self, node: Node) -> Option<usize> {
        let result = self.data.binary_search_by_key(&node, |entry| entry.0);
        match result {
            Ok(index) => Some(index),
            Err(_) => None,
        }
    }
}

impl<D> LogData<D> for SparseLogData<D>
where
    D: SupportedDigest,
{
    fn has_hash(&self, node: Node) -> bool {
        self.get_index(node).is_some()
    }

    fn hash_for(&self, node: Node) -> Option<Hash<D>> {
        let index = self.get_index(node)?;
        Some(self.data[index].1.clone())
    }
}
