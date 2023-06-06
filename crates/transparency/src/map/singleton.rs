use std::{
    error::Error,
    fmt,
    fs::{self, File},
    io::Write,
    sync::Arc,
};

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
    pub height: usize,
    pub side: Side,
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
    pub fn new(key: Hash<D>, value: Hash<D>, height: usize, side: Side) -> Self {
        Self {
            key,
            value,
            height,
            side,
        }
    }

    pub fn key(&self) -> Hash<D> {
        self.key.clone()
    }

    pub fn hash(&self) -> Hash<D> {
        let mut hash = self.value.clone();
        let mut reversed = ReversePath::new(self.key.clone());
        for n in 0..self.height {
            hash = match reversed.next() {
                Some(side) => match side {
                    Side::Left => hash_branch(hash.clone(), D::empty_tree_hash(n)),
                    Side::Right => hash_branch(D::empty_tree_hash(n), hash.clone()),
                },
                None => hash,
            };
        }
        hash
    }
}

impl<D: SupportedDigest> Clone for Singleton<D> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            value: self.value.clone(),
            height: self.height,
            side: self.side,
        }
    }
}
