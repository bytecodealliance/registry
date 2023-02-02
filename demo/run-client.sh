alias warg=../target/debug/warg

export WARG_DEMO_USER_KEY="ecdsa-p256:2CV1EpLaSYEn4In4OAEDAj5O4Hzu8AFAxgHXuG310Ew="

warg set-registry http://127.0.0.1:8090

warg publish start --name grep --init
warg publish release 1.0.0 --path ./simple-grep-1.0.0.wasm
warg publish submit

warg install grep
cat dummy-log.txt | warg run grep WARNING

read -p "Press [Enter] to proceed"

warg update
cat dummy-log.txt | warg run grep WARNING

