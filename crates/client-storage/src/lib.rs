use storage::ClientStorage;

mod storage;

struct Component;

impl bindings::reg::Reg for Component {
    fn hello_world() -> String {
        // bindings::store_registry_info("http://127.0.0.1:8090");
        "Hello, World!".to_string()
    }

    fn passthrough() -> String {
      bindings::storage::store_registry_info("http://127.0.0.1:8090");
      return bindings::storage::get_registry_info();
    }

    fn get_registry_pass() -> String {
      return bindings::storage::get_registry_info();
    }

    fn update() {
      let checkpoint = bindings::storage::get_checkpoint();
      let root = bindings::storage::hash_checkpoint(bindings::storage::CheckpointParam {
        contents: bindings::storage::ContentParam {
          log_root: &checkpoint.contents.log_root,
          log_length: checkpoint.contents.log_length,
          map_root: &checkpoint.contents.map_root
        },
        key_id: &checkpoint.key_id,
        signature: &checkpoint.signature
      });
      println!("THE CHECKPOINT {:?}", checkpoint);
    }
}

// et mut client = self.common.create_client()?;

//         match client.storage().load_registry_info().await? {
//             Some(_) => bail!("registry has already been initialized"),
//             None => {
//                 client
//                     .storage()
//                     .store_registry_info(&RegistryInfo {
//                         url: self.registry,
//                         checkpoint: None,
//                     })
//                     .await
//             }
//         }

bindings::export!(Component);
