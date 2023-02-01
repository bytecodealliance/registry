use digest::generic_array::GenericArray;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use std::fmt;

use crate::{ByteVisitor, VisitBytes};

use super::{Output, SupportedDigest};

#[derive(PartialOrd, Ord)]
pub struct Hash<D: SupportedDigest> {
    pub(crate) digest: Output<D>,
}

struct HashVisitor<D: SupportedDigest> {
    digest: D,
}

impl<D> HashVisitor<D>
where
    D: SupportedDigest,
{
    fn new() -> Self {
        HashVisitor { digest: D::new() }
    }

    fn finalize(self) -> Hash<D> {
        Hash {
            digest: self.digest.finalize(),
        }
    }
}

impl<D: SupportedDigest> ByteVisitor for HashVisitor<D> {
    fn visit_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        self.digest.update(bytes)
    }
}

impl<D: SupportedDigest> Hash<D> {
    pub fn of(content: impl VisitBytes) -> Self {
        let mut visitor = HashVisitor::new();
        content.visit(&mut visitor);
        visitor.finalize()
    }

    pub fn bytes(&self) -> &[u8] {
        self.digest.as_slice()
    }

    pub fn len(&self) -> usize {
        self.bytes().len()
    }
}

impl<D: SupportedDigest> VisitBytes for Hash<D> {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        visitor.visit_bytes(self.bytes())
    }
}

impl<D: SupportedDigest> std::hash::Hash for Hash<D> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.digest.hash(state);
    }
}

// Derived clone does not have precise enough bounds and type info.
impl<D: SupportedDigest> Clone for Hash<D> {
    fn clone(&self) -> Self {
        Self {
            digest: self.digest.clone(),
        }
    }
}

impl<D: SupportedDigest> Eq for Hash<D> {}
impl<D: SupportedDigest> PartialEq for Hash<D> {
    fn eq(&self, other: &Self) -> bool {
        self.digest == other.digest
    }
}

impl<D: SupportedDigest> fmt::Display for Hash<D> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}:{}",
            D::ALGORITHM,
            hex::encode(self.digest.as_slice())
        )
    }
}

impl<D: SupportedDigest> fmt::Debug for Hash<D> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Hash<{:?}>({})",
            D::ALGORITHM,
            hex::encode(self.digest.as_slice())
        )
    }
}

impl<D: SupportedDigest> TryFrom<Vec<u8>> for Hash<D> {
    type Error = IncorrectLengthError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let hash = Hash {
            digest: GenericArray::from_exact_iter(value.into_iter()).ok_or(IncorrectLengthError)?,
        };
        Ok(hash)
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("The provided vector was not the correct length")]
pub struct IncorrectLengthError;

impl<D: SupportedDigest> Serialize for Hash<D> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.digest)
    }
}

impl<'de, T: SupportedDigest> Deserialize<'de> for Hash<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CopyVisitor<T>(T);

        impl<T: AsRef<[u8]> + AsMut<[u8]>> From<T> for CopyVisitor<T> {
            fn from(buffer: T) -> Self {
                Self(buffer)
            }
        }

        impl<'a, T: AsRef<[u8]> + AsMut<[u8]>> Visitor<'a> for CopyVisitor<T> {
            type Value = T;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_fmt(format_args!("{} bytes", self.0.as_ref().len()))
            }

            #[cfg(feature = "alloc")]
            fn visit_byte_buf<E: Error>(self, v: alloc::vec::Vec<u8>) -> Result<Self::Value, E> {
                self.visit_bytes(&v)
            }

            fn visit_borrowed_bytes<E: Error>(self, v: &'a [u8]) -> Result<Self::Value, E> {
                self.visit_bytes(v)
            }

            fn visit_bytes<E: Error>(mut self, v: &[u8]) -> Result<Self::Value, E> {
                if v.len() != self.0.as_mut().len() {
                    return Err(E::custom("invalid length"));
                }

                self.0.as_mut().copy_from_slice(v);
                Ok(self.0)
            }
        }

        let buffer = Output::<T>::default();
        let visitor = CopyVisitor::from(buffer);
        Ok(Self {
            digest: deserializer.deserialize_bytes(visitor)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use sha2::Sha256;

    use super::*;

    #[test]
    fn test_hash_empties_have_no_impact() {
        let empty: &[u8] = &[];

        let h0: Hash<Sha256> = Hash::of(&(0u8, 1u8));
        let h1: Hash<Sha256> = Hash::of(&(0u8, 1u8, empty));
        let h2: Hash<Sha256> = Hash::of(&(0u8, empty, 1u8));
        let h3: Hash<Sha256> = Hash::of(&(0u8, empty, 1u8, empty));
        let h4: Hash<Sha256> = Hash::of(&(empty, 0u8, 1u8));
        let h5: Hash<Sha256> = Hash::of(&(empty, 0u8, 1u8, empty));
        let h6: Hash<Sha256> = Hash::of(&(empty, 0u8, empty, 1u8));
        let h7: Hash<Sha256> = Hash::of(&(empty, 0u8, empty, 1u8, empty));

        assert_eq!(h0, h1);
        assert_eq!(h0, h2);
        assert_eq!(h0, h3);
        assert_eq!(h0, h4);
        assert_eq!(h0, h5);
        assert_eq!(h0, h6);
        assert_eq!(h0, h7);
    }
}
