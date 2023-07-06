fn main() {
    // Re-run diesel_migrations::embed_migrations! on change
    println!("cargo:rerun-if-changed=src/datastore/postgres/migrations")
}
