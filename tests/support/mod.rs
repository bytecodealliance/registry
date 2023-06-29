use anyhow::{bail, Context, Result};
use std::{
    env,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use tokio::{fs, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use url::Url;
use warg_client::{
    storage::{ContentStorage, PublishEntry, PublishInfo},
    FileSystemClient, StorageLockResult,
};
use warg_crypto::{
    hash::AnyHash,
    signing::{KeyID, PrivateKey},
};
use warg_protocol::registry::PackageId;
use warg_server::{
    datastore::DataStore,
    policy::{content::WasmContentPolicy, record::AuthorizedKeyPolicy},
    Config, Server,
};
use wit_parser::{Resolve, UnresolvedPackage};

pub fn test_operator_key() -> PrivateKey {
    let key = "ecdsa-p256:I+UlDo0HxyBBFeelhPPWmD+LnklOpqZDkrFP5VduASk=";
    PrivateKey::decode(key.to_string()).unwrap()
}

pub fn test_signing_key() -> PrivateKey {
    let key = "ecdsa-p256:2CV1EpLaSYEn4In4OAEDAj5O4Hzu8AFAxgHXuG310Ew=";
    PrivateKey::decode(key.to_string()).unwrap()
}

pub fn create_client(config: &warg_client::Config) -> Result<FileSystemClient> {
    match FileSystemClient::try_new_with_config(None, config)? {
        StorageLockResult::Acquired(client) => Ok(client),
        _ => bail!("failed to acquire storage lock"),
    }
}

pub struct ServerInstance {
    task: Option<JoinHandle<()>>,
    shutdown: CancellationToken,
}

impl Drop for ServerInstance {
    fn drop(&mut self) {
        futures::executor::block_on(async move {
            self.shutdown.cancel();
            self.task.take().unwrap().await.ok();
        });
    }
}

pub async fn root() -> Result<PathBuf> {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
    std::thread_local! {
        static TEST_ID: usize = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    }

    let id = TEST_ID.with(|n| *n);

    let mut path = env::current_exe()?;
    path.pop(); // remove test exe name
    path.pop(); // remove `deps`
    path.pop(); // remove `debug` or `release`
    path.push("tests");
    path.push(
        std::env::current_exe()
            .context("failed to get process name")?
            .file_name()
            .context("failed to get process name")?
            .to_str()
            .context("failed to get process name")?,
    );
    path.push(format!("{id}"));

    fs::remove_dir_all(&path).await.ok();

    let server_content_dir = path.join("server");
    fs::create_dir_all(&server_content_dir).await?;

    let registries_dir = path.join("registries");
    fs::create_dir_all(&registries_dir).await?;

    let content_dir = path.join("content");
    fs::create_dir_all(&content_dir).await?;

    Ok(path)
}

/// Spawns a server as a background task.
pub async fn spawn_server(
    root: &Path,
    content_base_url: Option<Url>,
    data_store: Option<Box<dyn DataStore>>,
    authorized_keys: Option<Vec<(String, KeyID)>>,
) -> Result<(ServerInstance, warg_client::Config)> {
    let shutdown = CancellationToken::new();
    let mut config = Config::new(test_operator_key(), root.join("server"))
        .with_addr(([127, 0, 0, 1], 0))
        .with_shutdown(shutdown.clone().cancelled_owned())
        .with_checkpoint_interval(Duration::from_millis(100))
        .with_content_policy(WasmContentPolicy::default()); // For the tests, we assume only wasm content is allowed.

    if let Some(content_url) = content_base_url {
        config = config.with_content_base_url(content_url);
    }

    if let Some(authorized_keys) = authorized_keys {
        let mut policy = AuthorizedKeyPolicy::new();
        for (namespace, key) in authorized_keys {
            policy = policy.with_namespace_key(namespace, key)?;
        }

        config = config.with_record_policy(policy);
    }

    if let Some(store) = data_store {
        config = config.with_boxed_data_store(store);
    }

    let mut server = Server::new(config);
    let addr = server.bind()?;

    let task = tokio::spawn(async move {
        server.run().await.unwrap();
    });

    let instance = ServerInstance {
        task: Some(task),
        shutdown,
    };

    let config = warg_client::Config {
        default_url: Some(format!("http://{addr}")),
        registries_dir: Some(root.join("registries")),
        content_dir: Some(root.join("content")),
    };

    Ok((instance, config))
}

pub async fn publish(
    client: &FileSystemClient,
    id: &PackageId,
    version: &str,
    content: Vec<u8>,
    init: bool,
    signing_key: &PrivateKey,
) -> Result<AnyHash> {
    let digest = client
        .content()
        .store_content(
            Box::pin(futures::stream::once(async move { Ok(content.into()) })),
            None,
        )
        .await?;

    let mut entries = Vec::with_capacity(2);
    if init {
        entries.push(PublishEntry::Init);
    }
    entries.push(PublishEntry::Release {
        version: version.parse().unwrap(),
        content: digest.clone(),
    });

    let record_id = client
        .publish_with_info(
            signing_key,
            PublishInfo {
                id: id.clone(),
                head: None,
                entries,
            },
        )
        .await?;

    client
        .wait_for_publish(id, &record_id, Duration::from_millis(100))
        .await?;

    Ok(digest)
}

pub async fn publish_component(
    client: &FileSystemClient,
    id: &PackageId,
    version: &str,
    wat: &str,
    init: bool,
    signing_key: &PrivateKey,
) -> Result<AnyHash> {
    publish(client, id, version, wat::parse_str(wat)?, init, signing_key).await
}

pub async fn publish_wit(
    client: &FileSystemClient,
    id: &PackageId,
    version: &str,
    wit: &str,
    init: bool,
    signing_key: &PrivateKey,
) -> Result<AnyHash> {
    let mut resolve = Resolve::new();
    let pkg = resolve.push(UnresolvedPackage::parse(Path::new("foo.wit"), wit)?)?;

    publish(
        client,
        id,
        version,
        wit_component::encode(&resolve, pkg)?,
        init,
        signing_key,
    )
    .await
}
