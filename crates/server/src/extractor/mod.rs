pub mod interfaces;
pub mod metadata;
use wasmparser::BinaryReaderError;

pub type ExtractionResult<T> = Result<T, BinaryReaderError>;

pub trait ExtractionStream: Send + Sync {
    type Target;
    fn extract(&mut self, bytes: &[u8]) -> ExtractionResult<Option<Self::Target>>;

    fn result(&self) -> Option<Self::Target>;
}

pub trait Extractor<T>: Send + Sync {
    fn new_extraction_stream(&self) -> ExtractionResult<Box<dyn ExtractionStream<Target = T>>>;
}
