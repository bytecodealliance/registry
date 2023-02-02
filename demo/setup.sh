pushd ../crates/warg-cli
cargo build
popd

pushd ../crates/warg-server
cargo build
popd

chmod +x ../target/debug/warg
chmod +x ../target/debug/warg-server

rm -r ./.warg
rm -r ./.server-content
mkdir .server-content
