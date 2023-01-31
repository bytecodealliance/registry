use anyhow::{Error, Result};
use indexmap::IndexMap;
use warg_client::api;
use warg_crypto::hash::{DynHash, Hash, Sha256};
use warg_protocol::{ProtoEnvelope, package, registry::{LogLeaf, LogId}};

use crate::data::CliData;

pub async fn install(data: CliData, name: String) -> Result<()> {
    let reg_info = match data.get_registry_info()? {
        Some(reg_info) => reg_info,
        None => return Err(Error::msg("Must have a registry set to install.")),
    };
    let client = api::Client::new(reg_info.url().to_owned());

    let root: Hash<Sha256> = Hash::of(reg_info.checkpoint().as_ref());
    let root: DynHash = root.into();
    let mut state = data.get_package_state(&name)?;
    let head = state.head().as_ref().map(|head| head.digest.clone());

    let packages = IndexMap::from([(name.clone(), head)]);
    let mut response = client
        .fetch_logs(api::FetchRequest {
            root,
            operator: None,
            packages,
        })
        .await?;

    let new_records = response
        .packages
        .remove(&name)
        .ok_or_else(|| Error::msg("Registry did not provide required package data"))?;

    for envelope in new_records {
        let envelope: ProtoEnvelope<package::PackageRecord> = envelope.try_into()?;
        let needed_content = state.validate(&envelope)?;
        for digest in needed_content {
            let content_destination = data.content_path(&digest);
            client.download_content(digest, &content_destination).await?;
        }
    }

    if let Some(head) = state.head() {
        let log_id = LogId::package_log::<Sha256>(&name);
        let record_id = head.digest.clone();
        let leaf = LogLeaf { log_id, record_id };
        client.prove_inclusion(reg_info.checkpoint().as_ref(), vec![leaf]).await?;
    } else {
        return Err(Error::msg("Cannot validate empty package logs currently"));
    }

    Ok(())
}
