use std::marker::PhantomData;

use alloc::vec::Vec;
use warg_crypto::{
    hash::{Hash, SupportedDigest},
    VisitBytes,
};

use super::{LogData, Node};

pub struct SparseLogData<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    data: Vec<(Node, Hash<D>)>,
    _value: PhantomData<V>,
}

impl<D, V> SparseLogData<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    fn get_index(&self, node: Node) -> Option<usize> {
        let result = self.data.binary_search_by_key(&node, |entry| entry.0);
        match result {
            Ok(index) => Some(index),
            Err(_) => None,
        }
    }
}

impl<D, V> LogData<D, V> for SparseLogData<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    fn has_hash(&self, node: Node) -> bool {
        self.get_index(node).is_some()
    }

    fn hash_for(&self, node: Node) -> Option<Hash<D>> {
        let index = self.get_index(node)?;
        Some(self.data[index].1.clone())
    }
}

impl<D, V> From<Vec<(Node, Hash<D>)>> for SparseLogData<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    fn from(value: Vec<(Node, Hash<D>)>) -> Self {
        Self {
            data: value,
            _value: PhantomData,
        }
    }
}
