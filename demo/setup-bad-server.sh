rm -r .server-content
mkdir .server-content

alias demo_server=./bad-server

export WARG_DEMO_OPERATOR_KEY="ecdsa-p256:I+UlDo0HxyBBFeelhPPWmD+LnklOpqZDkrFP5VduASk="

demo_server --content-dir .server-content
