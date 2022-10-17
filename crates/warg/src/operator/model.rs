use std::str::FromStr;

use crate::things::{hash, signing::Key};

pub enum Permission {
    UpdateAuth,
}

impl ToString for Permission {
    fn to_string(&self) -> String {
        match self {
            Permission::UpdateAuth => "update-auth".to_string(),
        }
    }
}

impl FromStr for Permission {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "update-auth" => Ok(Permission::UpdateAuth),
            _ => Err(()),
        }
    }
}

pub enum Entry {
    Init {
        key: Key,
        hash_algorithm: hash::Algorithm,
    },
    UpdateAuth {
        key: Key,
        allow: Vec<Permission>,
        deny: Vec<Permission>,
    },
}
