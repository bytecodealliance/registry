# Warg Server

> ⚠️ This is prototype quality code at this time. ⚠️

```console
$ mkdir content
$ cargo run -- --content-dir content
2023-01-05T20:23:42.273099Z  INFO warg_server: Listening on 127.0.0.1:8090
```

```console
$ curl -v localhost:8090/content --data 'payload'
...
< HTTP/1.1 200 OK
< location: /content/sha256:239f59ed55e737c77147cf55ad0c1b030b6d7ee748a7426952f9b852d5a935e5
...
$ curl localhost:8090/content/sha256:239f59ed55e737c77147cf55ad0c1b030b6d7ee748a7426952f9b852d5a935e5
payload
```
