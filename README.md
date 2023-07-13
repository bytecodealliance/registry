# Preview registry

## Initial setup

> Beware: documented retroactively with slightly different steps

```console
$ fly apps create ba-preview-registry --org bytecode-alliance
$ fly postgres create --name preview-registry-db --org bytecode-alliance
$ fly postgres attach preview-registry-db --app ba-preview-registry --variable-name WARG_DATABASE_URL
$ fly secrets set WARG_OPERATOR_KEY=...
```

## Deploy

```console
$ docker buildx bake --push
$ fly deploy
```
