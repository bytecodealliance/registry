use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use super::{Output, SupportedDigest};

#[derive(Hash, PartialOrd, Ord)]
pub struct Hash<D: SupportedDigest> {
    pub(crate) digest: Output<D>,
}

impl<D: SupportedDigest> From<Output<D>> for Hash<D> {
    fn from(digest: Output<D>) -> Self {
        Hash { digest }
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
            "Hash<{}>({})",
            D::ALGORITHM,
            hex::encode(self.digest.as_slice())
        )
    }
}

impl<D: SupportedDigest> Deref for Hash<D> {
    type Target = Output<D>;

    fn deref(&self) -> &Self::Target {
        &self.digest
    }
}

impl<D: SupportedDigest> DerefMut for Hash<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.digest
    }
}

impl<D: SupportedDigest, U: ?Sized> AsRef<U> for Hash<D>
where
    Output<D>: AsRef<U>,
{
    fn as_ref(&self) -> &U {
        self.digest.as_ref()
    }
}

impl<D: SupportedDigest, U> AsMut<U> for Hash<D>
where
    Output<D>: AsMut<U>,
{
    fn as_mut(&mut self) -> &mut U {
        self.digest.as_mut()
    }
}

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
