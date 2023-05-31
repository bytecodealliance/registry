use super::{ContentPolicy, ContentPolicyError, ContentPolicyResult, ContentStreamPolicy};
use warg_crypto::hash::AnyHash;
use wasmparser::{
    Chunk, Encoding, FuncValidatorAllocations, Parser, ValidPayload, Validator, WasmFeatures,
};

/// A policy that ensures all uploaded content is valid WebAssembly.
pub struct WasmContentPolicy {
    allow_modules: bool,
    allow_components: bool,
    features: WasmFeatures,
}

impl WasmContentPolicy {
    /// Creates a new WebAssembly content policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Disallows WebAssembly modules from being acceptable content.
    pub fn disallow_modules(mut self) -> Self {
        self.allow_modules = false;
        self
    }

    /// Disallows WebAssembly components from being acceptable content.
    pub fn disallow_components(mut self) -> Self {
        self.allow_components = false;
        self
    }

    /// Sets the WebAssembly features to use when validating content.
    pub fn with_features(mut self, mut features: WasmFeatures) -> Self {
        // Always allow the component model feature
        features.component_model = true;
        self.features = features;
        self
    }
}

impl Default for WasmContentPolicy {
    fn default() -> Self {
        Self {
            allow_modules: true,
            allow_components: true,
            features: WasmFeatures {
                component_model: true,
                ..Default::default()
            },
        }
    }
}

impl ContentPolicy for WasmContentPolicy {
    fn new_stream_policy(
        &self,
        _digest: &AnyHash,
    ) -> ContentPolicyResult<Box<dyn ContentStreamPolicy>> {
        Ok(Box::new(WasmContentStreamPolicy {
            buffer: Vec::new(),
            parser: Parser::new(0),
            stack: Vec::new(),
            validator: wasmparser::Validator::new_with_features(self.features),
            allocs: FuncValidatorAllocations::default(),
            allow_modules: self.allow_modules,
            allow_components: self.allow_components,
        }))
    }
}

struct WasmContentStreamPolicy {
    buffer: Vec<u8>,
    parser: Parser,
    stack: Vec<Parser>,
    validator: Validator,
    allocs: FuncValidatorAllocations,
    allow_modules: bool,
    allow_components: bool,
}

impl WasmContentStreamPolicy {
    fn process(&mut self, bytes: &[u8], eof: bool) -> ContentPolicyResult<()> {
        // Extend the buffer if we need to; otherwise, parse the given slice
        let buf = if !self.buffer.is_empty() {
            self.buffer.extend(bytes);
            self.buffer.as_slice()
        } else {
            bytes
        };

        let mut offset = 0;
        loop {
            let (payload, consumed) = match self.parser.parse(&buf[offset..], eof).map_err(|e| {
                ContentPolicyError::Rejection(format!("content is not valid WebAssembly: {e}"))
            })? {
                Chunk::NeedMoreData(_) => {
                    // If the buffer is empty and there's still data in the given slice,
                    // copy the remaining data to the buffer.
                    // If there's still data remaining in the buffer, copy it to the
                    // beginning of the buffer and truncate it.
                    // Otherwise, clear the buffer.
                    if self.buffer.is_empty() && offset < bytes.len() {
                        self.buffer.extend_from_slice(&bytes[offset..]);
                    } else if offset < self.buffer.len() {
                        self.buffer.copy_within(offset.., 0);
                        self.buffer.truncate(self.buffer.len() - offset);
                    } else {
                        self.buffer.clear();
                    }
                    return Ok(());
                }

                Chunk::Parsed { consumed, payload } => (payload, consumed),
            };

            offset += consumed;

            match &payload {
                wasmparser::Payload::Version {
                    encoding: Encoding::Module,
                    ..
                } if !self.allow_modules => {
                    return Err(ContentPolicyError::Rejection(
                        "WebAssembly modules are not allowed".to_string(),
                    ))
                }
                wasmparser::Payload::Version {
                    encoding: Encoding::Component,
                    ..
                } if !self.allow_components => {
                    return Err(ContentPolicyError::Rejection(
                        "WebAssembly components are not allowed".to_string(),
                    ))
                }
                _ => {}
            }

            match self.validator.payload(&payload).map_err(|e| {
                ContentPolicyError::Rejection(format!("content is not valid WebAssembly: {e}"))
            })? {
                ValidPayload::Ok => {}
                ValidPayload::Parser(p) => {
                    self.stack.push(self.parser.clone());
                    self.parser = p;
                }
                ValidPayload::Func(func, body) => {
                    let allocs = std::mem::take(&mut self.allocs);
                    let mut validator = func.into_validator(allocs);
                    validator.validate(&body).map_err(|e| {
                        ContentPolicyError::Rejection(format!(
                            "content is not valid WebAssembly: {e}"
                        ))
                    })?;
                    self.allocs = validator.into_allocations();
                }
                ValidPayload::End(_) => {
                    if let Some(parser) = self.stack.pop() {
                        self.parser = parser;
                    } else {
                        return Ok(());
                    }
                }
            }
        }
    }
}

impl ContentStreamPolicy for WasmContentStreamPolicy {
    fn check(&mut self, bytes: &[u8]) -> ContentPolicyResult<()> {
        self.process(bytes, false)
    }

    fn finalize(&mut self) -> ContentPolicyResult<()> {
        self.process(&[], true)
    }
}
