//! "Dead Simple Signing Envelope"
//! https://github.com/secure-systems-lab/dsse

#![allow(dead_code)]

use std::io::Write;

use serde::{Deserialize, Serialize};
use signature::{Signer, Verifier};

use crate::Error;

/// DSSE Envelope
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    payload_type: String,
    #[serde(with = "crate::serde::base64")]
    payload: Vec<u8>,
    signatures: Vec<Signature>,
}

impl Envelope {
    pub fn new(payload_type: String, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            payload_type,
            payload: payload.into(),
            signatures: vec![],
        }
    }

    pub fn signatures(&self) -> impl Iterator<Item = &Signature> {
        self.signatures.iter()
    }

    pub fn sign<S: signature::Signature>(
        &mut self,
        signer: impl Signer<S>,
        key_id: Option<String>,
    ) -> Result<(), Error> {
        self.signatures.push(Signature::sign_payload(
            &self.payload_type,
            &self.payload,
            &signer,
            key_id,
        )?);
        Ok(())
    }

    /// Attempt to verify the first Signature matching the given `key_id`.
    pub fn verify<S: signature::Signature>(
        &self,
        verifier: impl Verifier<S>,
        key_id: Option<&str>,
    ) -> Result<&[u8], Error> {
        let signature = self
            .signatures()
            .find(|sig| sig.key_id.as_deref() == key_id)
            .ok_or_else(|| {
                Error::InvalidSignatureKey(
                    format!("no signature found with key_id {:?}", key_id).into(),
                )
            })?;
        signature.verify_payload(&self.payload_type, &self.payload, verifier)?;
        Ok(&self.payload)
    }
}

/// DSSE Signature
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signature {
    #[serde(rename = "sig", with = "crate::serde::base64")]
    signature: Vec<u8>,

    /// "unauthenticated hint indicating what key and algorithm was used to sign the message"
    #[serde(rename = "keyid", skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
}

impl Signature {
    pub fn sign_payload<S: signature::Signature>(
        payload_type: &str,
        payload: &[u8],
        signer: &impl Signer<S>,
        key_id: Option<String>,
    ) -> Result<Self, Error> {
        let msg = pre_authentication_encoding(payload_type.as_bytes(), payload);
        let signature = signer.try_sign(&msg)?.as_bytes().into();
        Ok(Self { signature, key_id })
    }

    pub fn verify_payload<S: signature::Signature>(
        &self,
        payload_type: &str,
        payload: &[u8],
        verifier: impl Verifier<S>,
    ) -> Result<(), Error> {
        let signature = S::from_bytes(&self.signature)?;
        let msg = pre_authentication_encoding(payload_type.as_bytes(), payload);
        verifier.verify(&msg, &signature)?;
        Ok(())
    }
}

// PAE(type, body) = "DSSEv1" + SP + LEN(type) + SP + type + SP + LEN(body) + SP + body
fn pre_authentication_encoding(type_: &[u8], body: &[u8]) -> Vec<u8> {
    // Rather than precisely calculating the size of the LEN fields, just over-allocate a little
    let mut buf = Vec::with_capacity(25 + type_.len() + body.len());
    buf.extend_from_slice(b"DSSEv1 ");
    write!(&mut buf, "{} ", type_.len()).unwrap();
    buf.extend_from_slice(type_);
    write!(&mut buf, " {} ", body.len()).unwrap();
    buf.extend_from_slice(body);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    // Spec tests from https://github.com/secure-systems-lab/dsse/blob/master/implementation/signing_spec.py

    const SPEC_KEY_ID: &str = "66301bbf";
    // hex(97358161215184420915383655311931858321456579547487070936769975997791359926199)
    const SPEC_KEY_HEX: &str = "d73ec437fd6346e3619c5ebfdfff0f6916804955ad32ac9ac492b0ede1f6ffb7";

    fn spec_key() -> p256::ecdsa::SigningKey {
        p256::ecdsa::SigningKey::from_bytes(&hex::decode(SPEC_KEY_HEX).unwrap()).unwrap()
    }

    fn spec_envelope() -> Envelope {
        serde_json::from_value(serde_json::json!({
            "payload": "aGVsbG8gd29ybGQ=",
            "payloadType": "http://example.com/HelloWorld",
            "signatures": [{
                "keyid": SPEC_KEY_ID,
                "sig": "A3JqsQGtVsJ2O2xqrI5IcnXip5GToJ3F+FnZ+O88SjtR6rDAajabZKciJTfUiHqJPcIAriEGAHTVeCUjW2JIZA==",
            }]
        }))
        .unwrap()
    }

    #[test]
    fn verify_spec_signature() {
        let envelope = spec_envelope();
        let payload = envelope
            .verify(spec_key().verifying_key(), Some(SPEC_KEY_ID))
            .expect("verify failed");

        assert_eq!(payload, b"hello world");
    }

    #[test]
    fn pae_spec() {
        assert_eq!(
            pre_authentication_encoding(b"http://example.com/HelloWorld", b"hello world"),
            b"DSSEv1 29 http://example.com/HelloWorld 11 hello world"
        );
    }

    #[test]
    fn round_trip() {
        let payload = b"Payload";

        let mut envelope = Envelope::new("RoundTrip".to_string(), payload.to_vec());

        let key_id = "KeyId";
        envelope
            .sign(spec_key(), Some(key_id.to_string()))
            .expect("sign failed");

        let json = serde_json::to_vec(&envelope).expect("serialize faile");
        let envelope: Envelope = serde_json::from_slice(&json).expect("deserialize failed");

        let verified_payload = envelope
            .verify(spec_key().verifying_key(), Some(key_id))
            .expect("verify failed");

        assert_eq!(verified_payload, payload);
    }
}
