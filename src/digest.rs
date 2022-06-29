use std::fmt::Display;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::Error;

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TypedDigest {
    Dummy(()), // FIXME(lann): remove or guard w/ feature flag
    Sha256(Sha256Digest),
}

impl Display for TypedDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypedDigest::Dummy(_) => write!(f, "dummy"),
            TypedDigest::Sha256(Sha256Digest(ref bytes)) => {
                write!(f, "sha256:{}", hex::encode(bytes))
            }
        }
    }
}

impl std::str::FromStr for TypedDigest {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "dummy" {
            Ok(Self::Dummy(()))
        } else if let Some(digest_str) = s.strip_prefix("sha256:") {
            Ok(TypedDigest::Sha256(Sha256Digest::try_from(digest_str)?))
        } else {
            Err(Error::InvalidContentDigest(
                "unrecognized digest type".into(),
            ))
        }
    }
}

#[derive(Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "&str")]
pub struct Sha256Digest(Vec<u8>);

impl Sha256Digest {
    pub fn digest(prefix: &'static [u8], data: impl AsRef<[u8]>) -> Self {
        let mut hasher = Sha256::new_with_prefix(prefix);
        hasher.update(data);
        Self((*hasher.finalize()).into())
    }

    pub async fn digest_read(mut r: impl AsyncRead + Unpin) -> tokio::io::Result<Self> {
        let mut hasher = Sha256::new();
        let mut buf = Vec::with_capacity(8 * 1024);
        loop {
            let n = r.read_buf(&mut buf).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buf);
            buf.clear();
        }
        Ok(Self((*hasher.finalize()).into()))
    }
}

impl std::fmt::Debug for Sha256Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Sha256Digest")
            .field(&hex::encode(&self.0))
            .finish()
    }
}

impl AsRef<[u8]> for Sha256Digest {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Sha256Digest> for String {
    fn from(digest: Sha256Digest) -> Self {
        hex::encode(digest.0)
    }
}

impl TryFrom<&str> for Sha256Digest {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let bytes = hex::decode(value)
            .map_err(|err| Error::InvalidContentDigest(err.to_string().into()))?;
        if bytes.len() != <Sha256 as Digest>::output_size() {
            return Err(Error::InvalidContentDigest("wrong digest size".into()));
        }
        Ok(Self(bytes))
    }
}
