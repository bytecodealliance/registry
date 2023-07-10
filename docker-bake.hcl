group "default" {
  targets = ["preview-registry"]
}

target "warg-server" {
  context = "https://github.com/bytecodealliance/registry.git"
  target = "warg-server"
  args = {
    FEATURES = "postgres,warg-server/debug"
  }
}

target "preview-registry" {
  contexts = { warg-server = "target:warg-server" }
  dockerfile-inline = <<-EOT
    FROM warg-server
    COPY authorized_keys.toml .
    ENV WARG_AUTHORIZED_KEYS_FILE=authorized_keys.toml
    COPY entrypoint.sh /usr/local/bin/
    ENTRYPOINT ["entrypoint.sh"]
    EOT
  tags = ["registry.fly.io/ba-preview-registry"]
}
