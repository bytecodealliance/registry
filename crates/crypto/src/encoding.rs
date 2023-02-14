use signature::Error as SignatureError;

use crate::signing;
use crate::{ByteVisitor, VisitBytes};

pub trait Encode {
    fn encode(&self) -> Vec<u8>;
}

#[derive(Default)]
struct EncodingVisitor {
    bytes: Vec<u8>,
}

impl ByteVisitor for EncodingVisitor {
    fn visit_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        self.bytes.extend(bytes.as_ref())
    }
}

impl<T> Encode for T
where
    T: VisitBytes,
{
    fn encode(&self) -> Vec<u8> {
        let mut visitor = EncodingVisitor::default();
        self.visit(&mut visitor);
        visitor.bytes
    }
}

pub trait Signable: Encode {
    const PREFIX: &'static [u8];

    fn sign(
        &self,
        private_key: &signing::PrivateKey,
    ) -> Result<signing::Signature, SignatureError> {
        let prefixed_content = [Self::PREFIX, b":", self.encode().as_slice()].concat();
        private_key.sign(&prefixed_content)
    }

    fn verify(
        public_key: &signing::PublicKey,
        msg: &[u8],
        signature: &signing::Signature,
    ) -> Result<(), SignatureError> {
        let prefixed_content = [Self::PREFIX, b":", msg].concat();
        public_key.verify(&prefixed_content, signature)
    }
}
