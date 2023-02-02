mkdir .attacker
pushd .attacker

alias warg=../../target/debug/warg

export WARG_DEMO_USER_KEY="ecdsa-p256:5MHeBDMzoTyD/n7MDSokpOj33oAa0AW1Xm83wyrM5t4="

warg set-registry http://127.0.0.1:8090
warg install grep

warg publish start --name grep
warg publish release 1.0.1 --path ../simple-grep-1.0.1.wasm
warg publish submit

popd
rm -rf .attacker