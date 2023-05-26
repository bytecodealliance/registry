use anyhow::{Context, Result};
use std::{
    env,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use tokio::{fs, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use warg_client::{
    storage::{ContentStorage, PublishEntry, PublishInfo},
    FileSystemClient,
};
use warg_server::{
    datastore::DataStore,
    policy::content::{ContentPolicyCollection, WasmContentPolicy},
    Config, Server,
};
use wit_parser::{Resolve, UnresolvedPackage};

pub fn test_operator_key() -> &'static str {
    "ecdsa-p256:I+UlDo0HxyBBFeelhPPWmD+LnklOpqZDkrFP5VduASk="
}

pub fn test_signing_key() -> &'static str {
    "ecdsa-p256:2CV1EpLaSYEn4In4OAEDAj5O4Hzu8AFAxgHXuG310Ew="
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

    let packages_dir = path.join("packages");
    fs::create_dir_all(&packages_dir).await?;

    let content_dir = path.join("content");
    fs::create_dir_all(&content_dir).await?;

    Ok(path)
}

/// Spawns a server as a background task.
pub async fn spawn_server(
    root: &Path,
    data_store: Option<Box<dyn DataStore>>,
) -> Result<(ServerInstance, warg_client::Config)> {
    // For the tests, we assume only wasm content is allowed.
    let mut policies = ContentPolicyCollection::default();
    policies.push(WasmContentPolicy::default());

    let shutdown = CancellationToken::new();
    let mut config = Config::new(test_operator_key().parse()?, root.join("server"))
        .with_addr(([127, 0, 0, 1], 0))
        .with_shutdown(shutdown.clone().cancelled_owned())
        .with_checkpoint_interval(Duration::from_millis(100))
        .with_content_policy(policies);

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
    name: &str,
    version: &str,
    content: Vec<u8>,
    init: bool,
) -> Result<()> {
    let digest = client
        .content()
        .store_content(
            Box::pin(futures::stream::once(async move { Ok(content.into()) })),
            None,
        )
        .await
        .context("failed to store component for publishing")?;

    let mut entries = Vec::with_capacity(2);
    if init {
        entries.push(PublishEntry::Init);
    }
    entries.push(PublishEntry::Release {
        version: version.parse().unwrap(),
        content: digest,
    });

    let record_id = client
        .publish_with_info(
            &test_signing_key().parse().unwrap(),
            PublishInfo {
                package: name.to_string(),
                head: None,
                entries,
            },
        )
        .await
        .context("failed to publish package")?;

    client
        .wait_for_publish(name, &record_id, Duration::from_millis(100))
        .await?;

    Ok(())
}

pub async fn publish_component(
    client: &FileSystemClient,
    name: &str,
    version: &str,
    wat: &str,
    init: bool,
) -> Result<()> {
    publish(
        client,
        name,
        version,
        wat::parse_str(wat).context("failed to parse component for publishing")?,
        init,
    )
    .await
}

pub async fn publish_wit(
    client: &FileSystemClient,
    name: &str,
    version: &str,
    wit: &str,
    init: bool,
) -> Result<()> {
    let mut resolve = Resolve::new();
    let pkg = resolve
        .push(
            UnresolvedPackage::parse(Path::new("foo.wit"), wit)
                .context("failed to parse wit for publishing")?,
            &Default::default(),
        )
        .context("failed to resolve wit for publishing")?;

    publish(
        client,
        name,
        version,
        wit_component::encode(&resolve, pkg).context("failed to encode wit for publishing")?,
        init,
    )
    .await
}
