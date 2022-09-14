//! A wrapper around the Digest Output<D> type for utility purposes
use core::fmt::{Debug, LowerHex};
use core::ops::{Deref, DerefMut};

use digest::{Digest, Output};
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A hash value produced using the Digest algorithm [D]
pub struct Hash<D: Digest>(Output<D>);

impl<D: Digest> Clone for Hash<D> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<D: Digest> Debug for Hash<D>
where
    Output<D>: LowerHex,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl<D: Digest> Eq for Hash<D> {}
impl<D: Digest> PartialEq for Hash<D> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<D: Digest> Ord for Hash<D> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<D: Digest> PartialOrd for Hash<D> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<D: Digest> core::hash::Hash for Hash<D> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<D: Digest> From<Output<D>> for Hash<D> {
    fn from(output: Output<D>) -> Self {
        Self(output)
    }
}

impl<D: Digest> Deref for Hash<D> {
    type Target = Output<D>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D: Digest> DerefMut for Hash<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<D: Digest, U: ?Sized> AsRef<U> for Hash<D>
where
    Output<D>: AsRef<U>,
{
    fn as_ref(&self) -> &U {
        self.0.as_ref()
    }
}

impl<D: Digest, U> AsMut<U> for Hash<D>
where
    Output<D>: AsMut<U>,
{
    fn as_mut(&mut self) -> &mut U {
        self.0.as_mut()
    }
}

impl<D: Digest> IntoIterator for Hash<D> {
    type Item = <Output<D> as IntoIterator>::Item;
    type IntoIter = <Output<D> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, D: Digest> IntoIterator for &'a Hash<D> {
    type Item = <&'a Output<D> as IntoIterator>::Item;
    type IntoIter = <&'a Output<D> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl<D: Digest> Serialize for Hash<D> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de, T: Digest> Deserialize<'de> for Hash<T> {
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
        Ok(Self(deserializer.deserialize_bytes(visitor)?))
    }
}
