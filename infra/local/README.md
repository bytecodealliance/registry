## Overview

This directory contains scripts and Docker Compose configuration to run the registry along with associated infra like
a database for storage for local development and testing purposes. The script names closely mirror the `docker-compose`
commands.

To help avoid conflicts with other processes that may be running on one's machine, the locally bound ports start with a
random number, 17513. This configuration along with other variables are set in `.env`.

### Usage

Start up the local infra:

```
./up.sh
```

Start the local infra while forcing a rebuild of source code:

```
./rebuild.sh
```

Both the above commands generate `*.local.*` files that provide assistance for interacting with the local infra:

- `pgpass.local.conf`, the `pgpass` configuration for connecting to the database
- `psql.local.sh`, a `psql` convenience wrapper script that uses the above `pgpass`


Stopping the local infra:

```
./stop.sh
```

Stopping the local infra and completely removing any resources:

```
./down.sh
```
