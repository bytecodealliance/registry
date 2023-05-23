# Warg Registry Server

> ⚠️ This is prototype quality code at this time. ⚠️

## Running the server

The registry server can be started with either in-memory or PostgreSQL storage.

### In-memory storage

With in-memory storage, the server will store all data in-memory and the data 
will be lost when the server is stopped.

To start the server, provide the `OPERATOR_KEY` environment variable, 
which is used to sign the entries in the server's operator log:

```console
$ OPERATOR_KEY="ecdsa-p256:I+UlDo0HxyBBFeelhPPWmD+LnklOpqZDkrFP5VduASk=" cargo run -- --content-dir content
2023-04-18T23:48:52.149746Z  INFO warg_server::services::core: initializing core service
2023-04-18T23:48:52.170199Z  INFO warg_server::services::core: core service is running
2023-04-18T23:48:52.170233Z  INFO warg_server: listening on 127.0.0.1:8090
```

### PostgreSQL storage

With PostgreSQL storage, the server will store all data in a PostgreSQL 
database. 

Support for PostgreSQL storage is behind the `postgres` compilation feature 
flag.

The easiest way to start a PostgreSQL server is with Docker:

```console
docker run -d --name postgres -e POSTGRES_PASSWORD=password -v /tmp/data:/var/lib/postgresql/data -p 5432:5432 postgres
```

With the above command, data will be stored in `/tmp/data` on the host machine.

To set up the database, install `diesel-cli`:

```console
cargo install diesel_cli
```

And run the setup with:

```console
diesel database setup --database-url postgres://postgres:password@localhost/registry
```

Here, `registry` is the database name that will be created.

To start the registry server, provide both the `OPERATOR_KEY` and 
`DATABASE_URL` environment variables:

```console
DATABASE_URL=postgres://postgres:password@localhost/registry OPERATOR_KEY="ecdsa-p256:I+UlDo0HxyBBFeelhPPWmD+LnklOpqZDkrFP5VduASk=" cargo run -p warg-server --features postgres -- --content-dir content --data-store postgres
```

The `--data-store postgres` flag starts the server with PostgreSQL data storage.

The server may now be restarted and will continue to use the same database.
