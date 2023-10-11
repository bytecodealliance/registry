use wasm_metadata::RegistryMetadata;
use wasmparser::{Chunk, Parser};

use super::{ExtractionResult, ExtractionStream, Extractor};

#[derive(Default)]
pub struct MetadataExtractor {}

impl Extractor<RegistryMetadata> for MetadataExtractor {
    fn new_extraction_stream(
        &self,
    ) -> ExtractionResult<Box<dyn ExtractionStream<Target = RegistryMetadata>>>
    where
        Self: Sized,
    {
        Ok(Box::new(MetadataStreamExtractor::new()))
    }
}

struct MetadataStreamExtractor {
    buffer: Vec<u8>,
    parser: Parser,
    stack: Vec<Parser>,
    metadata: Option<RegistryMetadata>,
}

impl ExtractionStream for MetadataStreamExtractor {
    type Target = RegistryMetadata;
    fn extract(
        &mut self,
        bytes: &[u8],
    ) -> std::result::Result<
        std::option::Option<wasm_metadata::RegistryMetadata>,
        wasmparser::BinaryReaderError,
    > {
        self.process(bytes, false)
    }

    fn result(&self) -> Option<RegistryMetadata> {
        self.metadata.clone()
    }
}
impl MetadataStreamExtractor {
    fn new() -> Self {
        Self {
            buffer: Vec::new(),
            parser: Parser::new(0),
            stack: Vec::new(),
            metadata: None,
        }
    }
    fn process(&mut self, bytes: &[u8], eof: bool) -> ExtractionResult<Option<RegistryMetadata>> {
        let buf = if !self.buffer.is_empty() {
            self.buffer.extend(bytes);
            &self.buffer
        } else {
            bytes
        };
        let mut offset = 0;
        let mut depth = 0;
        loop {
            let (payload, consumed) = match self.parser.parse(&buf[offset..], eof) {
                Err(e) => {
                    return Err(e);
                }
                Ok(Chunk::NeedMoreData(_)) => {
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
                    return Ok(None);
                }

                Ok(Chunk::Parsed { consumed, payload }) => (payload, consumed),
            };
            offset += consumed;

            match &payload {
                wasmparser::Payload::ModuleSection { parser, .. } => {
                    self.stack.push(self.parser.clone());
                    self.parser = parser.clone();
                    depth += 1
                }
                wasmparser::Payload::ComponentSection { parser, .. } => {
                    self.stack.push(self.parser.clone());
                    self.parser = parser.clone();
                    depth += 1
                }
                wasmparser::Payload::End { .. } => {
                    if let Some(parser) = self.stack.pop() {
                        self.parser = parser;
                        depth -= 1
                    } else {
                        return Ok(None);
                    }
                }
                wasmparser::Payload::CustomSection(c)
                    if c.name() == "registry-metadata" && depth <= 0 =>
                {
                    let registry = RegistryMetadata::from_bytes(c.data(), 0).unwrap();
                    self.metadata = Some(registry.clone());
                    return Ok(Some(registry));
                }
                _ => {}
            }
        }
    }
}
