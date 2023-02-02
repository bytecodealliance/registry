use anyhow::{Error, Result};
use indexmap::IndexMap;
use warg_client::api;
use warg_crypto::hash::{DynHash, Hash, Sha256};
use warg_protocol::{ProtoEnvelope, package, registry::{LogLeaf, LogId, MapCheckpoint}, SerdeEnvelope};

use crate::data::CliData;

pub async fn update(data: CliData) -> Result<()> {
    let mut reg_info = match data.get_registry_info()? {
        Some(reg_info) => reg_info,
        None => return Err(Error::msg("Must have a registry set to install.")),
    };
    let client = api::Client::new(reg_info.url().to_owned());
    let checkpoint = client.latest_checkpoint().await?;
    update_with_client(&data, client, &checkpoint).await?;
    reg_info.set_checkpoint(checkpoint);
    data.set_registry_info(&reg_info)?;
    Ok(())
}

pub async fn update_to(data: &CliData, checkpoint: &SerdeEnvelope<MapCheckpoint>) -> Result<()> {
    let reg_info = match data.get_registry_info()? {
        Some(reg_info) => reg_info,
        None => return Err(Error::msg("Must have a registry set to install.")),
    };
    let client = api::Client::new(reg_info.url().to_owned());
    update_with_client(data, client, checkpoint).await
}

async fn update_with_client(data: &CliData, client: api::Client, checkpoint: &SerdeEnvelope<MapCheckpoint>) -> Result<()> {
    let root: Hash<Sha256> = Hash::of(checkpoint.as_ref());
    let root: DynHash = root.into();

    let mut validators = Vec::new();
    let mut packages = Vec::new();
    for (name, state) in data.get_all_packages()? {
        let head = state.head().as_ref().map(|head| head.digest.clone());
        validators.push((name.clone(), state));
        packages.push((name, head));
    }

    let packages = IndexMap::from_iter(packages.into_iter());
    let mut response = client
        .fetch_logs(api::FetchRequest {
            root,
            operator: None,
            packages,
        })
        .await?;

    let mut heads = Vec::new();
    for (name, state) in validators.iter_mut() {
        let new_records = response
            .packages
            .remove(name)
            .ok_or_else(|| Error::msg("Registry did not provide required package data"))?;

        for envelope in new_records {
            let envelope: ProtoEnvelope<package::PackageRecord> = envelope.try_into()?;
            let needed_content = state.validate(&envelope)?;
            for digest in needed_content {
                let content_destination = data.content_path(&digest);
                if !content_destination.exists() {
                    let tmp_path = client.download_content(digest, &data.temp_dir()).await?;
                    tmp_path.persist(&content_destination)?;
                }
            }
        }

        if let Some(head) = state.head() {
            let log_id = LogId::package_log::<Sha256>(&name);
            let record_id = head.digest.clone();
            let leaf = LogLeaf { log_id, record_id };
            heads.push(leaf);
        } else {
            return Err(Error::msg("Cannot validate empty package logs currently"));
        }
    }

    client.prove_inclusion(checkpoint.as_ref(), heads).await?;

    for (name, state) in validators.iter() {
        data.set_package_state(&name, &state)?;
    }

    Ok(())
}

