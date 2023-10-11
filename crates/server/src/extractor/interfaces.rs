use core::fmt;
use std::fmt::Display;

use wasmparser::{Chunk, ComponentExport, ComponentExternName, ComponentImport, Parser};

use crate::datastore::Direction;

use super::{ExtractionResult, ExtractionStream, Extractor};

#[derive(Debug, Clone)]
pub struct Interface {
    pub name: String,
    pub direction: Direction,
}

impl Display for Interface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.name, self.direction)
    }
}
#[derive(Default)]
pub struct InterfaceExtractor {}

impl Extractor<Vec<Interface>> for InterfaceExtractor {
    fn new_extraction_stream(
        &self,
    ) -> ExtractionResult<Box<dyn ExtractionStream<Target = Vec<Interface>>>>
    where
        Self: Sized,
    {
        Ok(Box::new(InterfaceStreamExtractor::new()))
    }
}

struct InterfaceStreamExtractor {
    buffer: Vec<u8>,
    parser: Parser,
    stack: Vec<Parser>,
    interfaces: Vec<Interface>,
}

impl ExtractionStream for InterfaceStreamExtractor {
    type Target = Vec<Interface>;
    fn extract(
        &mut self,
        bytes: &[u8],
    ) -> std::result::Result<std::option::Option<Vec<Interface>>, wasmparser::BinaryReaderError>
    {
        self.process(bytes, false)
    }

    fn result(&self) -> Option<Vec<Interface>> {
        if self.interfaces.len() > 0 {
            Some(self.interfaces.clone())
        } else {
            None
        }
    }
}
impl InterfaceStreamExtractor {
    fn new() -> Self {
        Self {
            buffer: Vec::new(),
            parser: Parser::new(0),
            stack: Vec::new(),
            interfaces: Vec::new(),
        }
    }
    fn process(&mut self, bytes: &[u8], eof: bool) -> ExtractionResult<Option<Vec<Interface>>> {
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
                    } else if depth <= 0 {
                        return Ok(Some(self.interfaces.clone()));
                    }
                }
                wasmparser::Payload::ComponentImportSection(s) => {
                    let iterable = s.clone().into_iter();
                    for sec in iterable {
                        let ComponentImport { name, .. } = sec?;
                        if let ComponentExternName::Interface(name) = name {
                            self.interfaces.push(Interface {
                                name: String::from(name),
                                direction: Direction::Export,
                            });
                        }
                    }
                }
                wasmparser::Payload::ComponentExportSection(s) => {
                    let iterable = s.clone().into_iter();
                    for sec in iterable {
                        let ComponentExport { name, .. } = sec?;
                        if let ComponentExternName::Interface(name) = name {
                            self.interfaces.push(Interface {
                                name: String::from(name),
                                direction: Direction::Export,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
