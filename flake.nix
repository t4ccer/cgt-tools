{
  description = "cgt-tools";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs?ref=nixos-unstable-small";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    pre-commit-hooks-nix = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = inputs @ {self, ...}:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [
        inputs.pre-commit-hooks-nix.flakeModule
      ];

      # `nix flake show --impure` hack
      systems =
        if builtins.hasAttr "currentSystem" builtins
        then [builtins.currentSystem]
        else inputs.nixpkgs.lib.systems.flakeExposed;

      perSystem = {
        config,
        pkgs,
        lib,
        system,
        ...
      }: let
        rustToolchain = pkgs.rust-bin.fromRustupToolchain {
          channel = "stable";
          components = ["rust-analyzer" "rust-src" "rustfmt" "rustc" "cargo"];
          targets = [
            "x86_64-unknown-linux-gnu"
            "x86_64-unknown-linux-musl"
            "wasm32-unknown-unknown"
          ];
        };

        pythonToolchain = pkgs.python313.override {
          packageOverrides = self: super: {
            anywidget = super.anywidget.overridePythonAttrs (oldAttrs: rec {
              version = "0.11.0";
              src = pkgs.fetchPypi {
                pname = "anywidget";
                inherit version;
                hash = "sha256-ZpX775RJz4wn9CG5bFg3qjf5CewfYM+jOt0zPhtwsWk=";
              };
            });
          };
        };
      in {
        _module.args.pkgs = import self.inputs.nixpkgs {
          inherit system;
          overlays = [
            inputs.rust-overlay.overlays.rust-overlay
          ];
        };

        pre-commit.settings = {
          src = ./.;
          hooks = {
            alejandra.enable = true;
            rustfmt = {
              enable = true;
              args = ["--style-edition=2024"];
            };
            typos = {
              enable = true;
              settings.ignored-words = [
                "nimber"
                "numer" # `numerator` from `num-rational`
              ];
            };
            taplo.enable = true;
            prettier.enable = true;
            trim-trailing-whitespace.enable = true;
          };
          tools = {
            rustfmt = lib.mkForce rustToolchain;
            clippy = lib.mkForce rustToolchain;
          };
        };

        devShells.default = pkgs.mkShell {
          shellHook = ''
            ${config.pre-commit.shellHook}
            PATH=$PATH:$(pwd)/target/release
          '';

          hardeningDisable = ["fortify"];

          nativeBuildInputs = [
            (pythonToolchain.withPackages (ps:
              with ps; [
                pip
                jupyter
                anywidget
                sphinx
                myst-parser
                furo
              ]))
            pkgs.maturin

            pkgs.cargo-expand
            pkgs.cargo-flamegraph
            pkgs.cargo-nextest
            pkgs.cargo-tarpaulin
            rustToolchain

            pkgs.alejandra
            pkgs.dot2tex
            pkgs.fd
            pkgs.graphviz
            pkgs.hyperfine
            pkgs.kdePackages.kcachegrind
            pkgs.lldb
            pkgs.texlive.combined.scheme-full
            pkgs.valgrind

            pkgs.wasm-pack
            pkgs.webpack-cli

            pkgs.pkg-config
            pkgs.SDL2
          ];
        };
        formatter = pkgs.alejandra;
      };
    };
}
