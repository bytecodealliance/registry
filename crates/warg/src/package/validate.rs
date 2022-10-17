use hashbrown::HashMap;

use crate::things::{envelope::Envelope, hash, signing::Key, Version};

use super::model::{Entry, Permission};

pub struct State {
    /// The hash is the hash of the key, which is used as the key id
    permissions: HashMap<hash::Hash, Vec<Permission>>,
    releases: HashMap<Version, ReleaseState>,
}

#[derive(Clone)]
pub enum ReleaseState {
    Unreleased,
    Released { content: hash::Hash },
    Yanked,
}

pub enum ValidationError {
    InitialEntryAfterBeginning,
    UnauthorizedAction {
        key: Key,
        needed_permission: Permission,
    },
    ReleaseOfReleased {
        version: Version,
    },
    ReleaseOfYanked {
        version: Version,
    },
    YankOfUnreleased {
        version: Version,
    },
    YankOfYanked {
        version: Version,
    },
}

pub fn validate_log(entries: &[Envelope<Entry>]) -> Result<Option<State>, ValidationError> {
    if entries.is_empty() {
        return Ok(None);
    }

    let mut entry_iter = entries.iter();

    let mut state = match entry_iter.next() {
        Some(entry) => validate_init(entry)?,
        None => return Ok(None),
    };

    for entry in entry_iter {
        state = validate_entry(state, entry)?;
    }

    Ok(Some(state))
}

pub fn validate_init(entry: &Envelope<Entry>) -> Result<State, ValidationError> {
    
}

pub fn validate_entry(mut state: State, entry: &Envelope<Entry>) -> Result<State, ValidationError> {
    let bytes = &entry.contents.bytes;
    let contents = &entry.contents.contents;

    // Check that the initial entry has not been repeated
    if matches!(entry.contents.contents, Entry::Init { .. }) {
        return Err(ValidationError::InitialEntryAfterBeginning);
    }

    // Check for permissions
    let needed_permission = contents.required_permission().unwrap();
    if let Some(available_permissions) = state.permissions.get(&key) {
        if !available_permissions.contains(&needed_permission) {
            return Err(ValidationError::UnauthorizedAction {
                key,
                needed_permission,
            });
        }
    } else {
        return Err(ValidationError::UnauthorizedAction {
            key,
            needed_permission,
        });
    }

    match entry {
        Entry::Init { .. } => unreachable!(),

        Entry::UpdateAuth { key, allow, deny } => {
            let mut new_permissions = Vec::new();

            // Keep any old permissions not denied
            if let Some(old_permissions) = state.permissions.get(&key) {
                for permission in old_permissions.iter() {
                    if !deny.contains(permission) {
                        new_permissions.push(*permission);
                    }
                }
            }
            // Add any new permissions allowed
            for permission in allow.iter() {
                new_permissions.push(*permission);
            }
            // Update permissions
            state.permissions.insert(key, new_permissions);

            Ok(state)
        }

        Entry::Release { version, content } => {
            let old_state = state
                .releases
                .get(&version)
                .cloned()
                .unwrap_or(ReleaseState::Unreleased);
            match old_state {
                ReleaseState::Unreleased => {
                    state
                        .releases
                        .insert(version, ReleaseState::Released { content });
                    Ok(state)
                }
                ReleaseState::Released { content: _ } => {
                    Err(ValidationError::ReleaseOfReleased { version })
                }
                ReleaseState::Yanked => Err(ValidationError::ReleaseOfYanked { version }),
            }
        }

        Entry::Yank { version } => {
            let old_state = state
                .releases
                .get(&version)
                .cloned()
                .unwrap_or(ReleaseState::Unreleased);
            match old_state {
                ReleaseState::Unreleased => Err(ValidationError::YankOfUnreleased { version }),
                ReleaseState::Released { content: _ } => {
                    state.releases.insert(version, ReleaseState::Yanked);
                    Ok(state)
                }
                ReleaseState::Yanked => Err(ValidationError::YankOfYanked { version }),
            }
        }
    }
}
