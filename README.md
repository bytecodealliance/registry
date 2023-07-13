# Preview registry

## Initial setup

```console
$ fly auth login
$ fly launch
...[interactive setup; mostly defaults with postgres]...
$ fly secrets set WARG_OPERATOR_KEY=... WARG_DATABASE_URL=...
```

## Deploy

```console
$ docker buildx bake --push
...
$ fly deploy
```
