rm -rf .server-content
mkdir .server-content

alias warg-server=../target/debug/warg-server
WARG_OPERATOR_KEY="ecdsa-p256:I+UlDo0HxyBBFeelhPPWmD+LnklOpqZDkrFP5VduASk=" warg-server --content-dir .server-content
