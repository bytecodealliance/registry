use storage::ClientStorage;

mod storage;

struct Component;

impl bindings::Component for Component {
    fn hello_world() -> String {
        "Hello, World!".to_string()
    }

    fn store_registry_info(input: String) {
      let store = storage::FileSystemStorage::new(".warg");
      match store {
        Ok(mut s) => {
          s.store_registry_info(&storage::RegistryInfo {url: input, checkpoint: None});
        },
        _ => {
          println!("ha");
        }
    }
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
