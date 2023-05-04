rm -rf .warg
alias warg=../target/debug/warg
warg config --overwrite --registry http://127.0.0.1:8090 --registries-dir .warg/registries --content-dir .warg/content warg-config.json
