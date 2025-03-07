installCargoToken() {
  mkdir -p ~/.cargo
  cat >~/.cargo/credentials.toml <<EOF
[registry]
token = "$(readSecretString cargo .${cargoSecretField:-token})"
EOF
}

preUserSetup+=("installCargoToken")
