use crate::things::hash::Hash;

pub struct Root {
    pub log_len: usize,
    pub log_root: Hash,
    pub map_root: Hash,
}
