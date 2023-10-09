use anyhow::{bail, Error};
use wasm_metadata::RegistryMetadata;
use wasmparser::{BinaryReaderError, Chunk, Parser};

use super::{ExtractionResult, ExtractionStream, Extractor};

pub struct MetadataExtractor {}
// pub trait MetadataExtractor: Send + Sync {}

impl MetadataExtractor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Extractor for MetadataExtractor {
    fn new_extraction_stream(&self) -> ExtractionResult<Box<dyn ExtractionStream>> {
        Ok(Box::new(MetadataStreamExtractor::new()))
    }
}

struct MetadataStreamExtractor {
    buffer: Vec<u8>,
    parser: Parser,
    stack: Vec<Parser>,
}

impl ExtractionStream for MetadataStreamExtractor {
    fn extract(&mut self, bytes: &[u8]) -> ExtractionResult<Option<RegistryMetadata>> {
        dbg!("PRE PROCESS");
        self.process(bytes, false)
    }

    fn result(&self) -> &[u8] {
        &self.buffer
    }
}
impl MetadataStreamExtractor {
    fn new() -> Self {
        Self {
            buffer: Vec::new(),
            parser: Parser::new(0),
            stack: Vec::new(),
        }
    }
    fn process(&mut self, bytes: &[u8], eof: bool) -> ExtractionResult<Option<RegistryMetadata>> {
        dbg!("PROCSSING");
        let buf = if !self.buffer.is_empty() {
            self.buffer.extend(bytes);
            &self.buffer
        } else {
            bytes
        };
        let mut offset = 0;
        let mut depth = 0;
        // let parser = &mut self.parser;
        loop {
            dbg!("RELOOPED");
            let (payload, consumed) = match self.parser.parse(&buf[offset..], eof)
            // .map_err(|e| {
              // ::Rejection(format!("content is not valid WebAssembly: {e}"))
              {
                // )
                Err(e) => {
                  // e
                  dbg!(e);
                  unreachable!()
                }
                Ok(Chunk::NeedMoreData(_)) => {
                    // If the buffer is empty and there's still data in the given slice,
                    // copy the remaining data to the buffer.
                    // If there's still data remaining in the buffer, copy it to the
                    // beginning of the buffer and truncate it.
                    // Otherwise, clear the buffer.
                    dbg!("NEEDED MORE DATA");
                    if self.buffer.is_empty() && offset < bytes.len() {
                        self.buffer.extend_from_slice(&bytes[offset..]);
                    } else if offset < self.buffer.len() {
                        self.buffer.copy_within(offset.., 0);
                        self.buffer.truncate(self.buffer.len() - offset);
                    } else {
                        self.buffer.clear();
                    }
                    // continue;
                    return Ok(None);
                    // unreachable!()
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
                    if c.name() == "registry-metadata" && depth == 0 =>
                {
                    dbg!("PARSE METADATA");
                    let registry = RegistryMetadata::from_bytes(&c.data(), 0).unwrap();
                    dbg!(&registry);
                    return Ok(Some(registry));
                }
                _ => {
                    dbg!(&payload);
                }
            }
        }
    }
}
