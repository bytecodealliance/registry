export WARG_DEMO_USER_KEY="ecdsa-p256:2CV1EpLaSYEn4In4OAEDAj5O4Hzu8AFAxgHXuG310Ew="

rm -rf .warg

alias warg-cli=../target/debug/warg-cli
warg-cli init http://127.0.0.1:8090
