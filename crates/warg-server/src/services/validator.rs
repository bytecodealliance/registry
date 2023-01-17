use std::sync::Arc;

use tokio::sync::mpsc::{self, Receiver};
use tokio::task::JoinHandle;

use warg_protocol::package::validate::Validator as PackageValidator;

use crate::policy::Policy;

use super::transparency::TransparencyService;
use super::package::{PackageService, PackageRecordStatus};
use super::PublishInfo;


pub struct ValidatorFactory {
    policy: Arc<Policy>,
    packager: Arc<PackageService>,
    recorder: Arc<TransparencyService>
}

impl ValidatorFactory {
    pub fn create(&self) -> ValidatorService {
        ValidatorService::new(self.packager.clone(), self.recorder.clone())
    }
}

pub struct ValidatorService {
    mailbox: mpsc::Sender<PublishInfo>,
    handle: JoinHandle<()>,
}

impl ValidatorService {
    pub fn new(packager: Arc<PackageService>, recorder: Arc<TransparencyService>) -> Self {
        let (mailbox, rx) = mpsc::channel::<PublishInfo>(4);
        let handle = tokio::spawn(async move {
            Self::process(rx, packager, recorder).await;
        });

        Self { mailbox, handle }
    }

    async fn process(mut rx: Receiver<PublishInfo>, packager: Arc<PackageService>, recorder: Arc<TransparencyService>) {
        let mut validator = PackageValidator::new();

        while let Some(info) = rx.recv().await {
            let backup = validator.clone();
            match validator.validate(&info.record) {
                Ok(_) => {
                    let status = PackageRecordStatus::Validated;
                    let package_id = info.package_id.clone();
                    let record_id = info.record_id.clone();
                    packager
                        .set_record_status(package_id.clone(), record_id.clone(), status)
                        .await;

                    recorder.record(package_id, record_id).await;
                },
                Err(error) => {
                    let status = super::package::PackageRecordStatus::Rejected {
                        reason: error.to_string(),
                    };
                    packager
                        .set_record_status(info.package_id, info.record_id, status)
                        .await;
                    validator = backup;
                }
            }
        }
    }
}

impl ValidatorService {
    pub async fn validate(&self, info: PublishInfo) {
        
    }
}
