pub mod package;
pub mod publish;
pub mod validator;
pub mod transparency;

use crate::api::package::ContentSource;
use warg_protocol::package::model::PackageRecord;
use warg_protocol::Envelope;

use warg_protocol::registry::{LogId, RecordId};

#[derive(Debug)]
pub struct PublishInfo {
    pub package_id: LogId,
    pub record_id: RecordId,
    pub record: Envelope<PackageRecord>,
    pub content_sources: Vec<ContentSource>,
}
