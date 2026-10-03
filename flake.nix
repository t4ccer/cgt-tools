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
        self',
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

        pythonToolchain = pkgs.python314.override {
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

        pkgsCuda = import inputs.nixpkgs {
          inherit system;
          config = {
            allowUnfree = true;
            cudaSupport = true;
          };
        };
        libtorch = pkgsCuda.libtorch-bin;

        pythonEnv = pythonToolchain.withPackages (ps:
          with ps; [
            pip
            jupyter
            anywidget
          ]);
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

        devShells = {
          default = pkgs.mkShell {
            shellHook = ''
              ${config.pre-commit.shellHook}
              PATH=$PATH:$(pwd)/target/release
              # Jupyter started from the venv only searches its own prefix for lab extensions,
              # so it would not find the widget manager installed in the nix python env
              export JUPYTER_PATH=${pythonEnv}/share/jupyter
              export CHROME=${pkgs.chromium}/bin/chromium
            '';

            hardeningDisable = ["fortify"];

            nativeBuildInputs = [
              pythonEnv
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
              pkgs.miniserve
              pkgs.chromium

              pkgs.pkg-config
              pkgs.SDL2
            ];
          };

          ai = self'.devShells.default.overrideAttrs (final: prev: {
            shellHook =
              (prev.shellHook or "")
              + ''
                export LD_LIBRARY_PATH=/run/opengl-driver/lib:${libtorch}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
              '';

            env =
              (prev.env or {})
              // {
                LIBTORCH = libtorch;
                LIBTORCH_INCLUDE = libtorch.dev;
                LIBTORCH_LIB = libtorch;
              };
          });
        };
        formatter = pkgs.alejandra;
      };
    };
}
