pub mod metadata;
use wasm_metadata::RegistryMetadata;
use wasmparser::BinaryReaderError;

pub type ExtractionResult<T> = Result<T, BinaryReaderError>;

pub trait ExtractionStream: Send + Sync {
    // type Target;
    fn extract(&mut self, bytes: &[u8]) -> ExtractionResult<Option<RegistryMetadata>>;

    fn result(&self) -> &[u8];
}

pub trait Extractor: Send + Sync {
    fn new_extraction_stream(&self) -> ExtractionResult<Box<dyn ExtractionStream>>;
    // fn new_extraction_stream<T>(&self) -> ExtractionResult<Box<dyn ExtractionStream<Target = T>>>;
}
