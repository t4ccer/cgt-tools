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
    };
    hercules-ci-effects = {
      url = "github:hercules-ci/hercules-ci-effects";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
    };
  };
  outputs = inputs @ {self, ...}:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} ({withSystem, ...}: let
      version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
    in {
      imports = [
        inputs.pre-commit-hooks-nix.flakeModule
        inputs.hercules-ci-effects.flakeModule
        ./nix/github-pages.nix
      ];

      # `nix flake show --impure` hack
      systems =
        if builtins.hasAttr "currentSystem" builtins
        then [builtins.currentSystem]
        else inputs.nixpkgs.lib.systems.flakeExposed;

      herculesCI = herculesArgs: {
        ciSystems = ["x86_64-linux"];
        onPush.cargo = {
          outputs.effects = withSystem "x86_64-linux" ({
            hci-effects,
            config,
            pkgs,
            ...
          }: {
            cargoPublish = let
              cargoSetupHook = pkgs.runCommand "hercules-ci-cargo-setup-hook" {} ''
                mkdir -p $out/nix-support
                cp ${./nix/cargo-setup-hook.sh} $out/nix-support/setup-hook
              '';

              cargoPublish = pkgs.callPackage ./nix/cargo-publish.nix {
                inherit (inputs.nixpkgs) lib;
                inherit (pkgs) cargo stdenv;
                inherit (hci-effects) mkEffect;
                inherit cargoSetupHook;
              };

              shouldRun =
                herculesArgs.config.repo.tag
                != null
                && (builtins.match "^v([0-9]+).([0-9]+).([0-9]+)$" herculesArgs.config.repo.tag) != null;
            in
              hci-effects.runIf shouldRun (cargoPublish {
                src = ./.;
              });
          });
        };
      };

      hercules-ci.github-releases.files = [
        {
          label = "cgt-tools-v${version}-x86_64-windows.zip";
          path = "${self.outputs.packages.x86_64-linux.cgt-tools-x86_64-windows-bundle}";
        }
      ];

      perSystem = {
        config,
        self',
        inputs',
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
          ];
        };

        pythonToolchain = "python311";

        hostPkgs = pkgs;

        mkCgtTools = {pkgs}:
          pkgs.rustPlatform.buildRustPackage {
            name = "cgt-tools";

            src = lib.cleanSourceWith {
              src = ./.;
              filter = name: type: let
                baseName = baseNameOf (toString name);
              in
                !((!lib.cleanSourceFilter name type)
                  || (baseName == "flake.lock")
                  || (lib.hasSuffix ".nix" baseName)
                  || (lib.hasSuffix ".md" baseName));
            };
            cargoLock.lockFile = ./Cargo.lock;

            cargoBuildFlags = ["-p cgt_gui -p cgt_cli"];

            buildInputs = [
              pkgs.SDL2
            ];

            doCheck = false;
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
          };
          tools = {
            rustfmt = lib.mkForce rustToolchain;
            clippy = lib.mkForce rustToolchain;
          };
        };

        packages = {
          cgt-tools-x86_64-windows = mkCgtTools {
            pkgs = pkgs.pkgsCross.mingwW64;
          };

          cgt-tools-x86_64-windows-bundle =
            pkgs.runCommand "cgt-tools-v${version}-x86_64-windows.zip" {
              nativeBuildInputs = [pkgs.zip];
            } ''
              cp --no-preserve=all -vLr ${self'.packages.cgt-tools-x86_64-windows}/bin/ ./cgt-tools-v${version}-x86_64-windows
              cp --no-preserve=all ${./LICENSE} ./cgt-tools-v${version}-x86_64-windows/LICENSE
              echo ${version} > ./cgt-tools-v${version}-x86_64-windows/VERSION
              zip -r cgt-tools-v${version}-x86_64-windows.zip cgt-tools-v${version}-x86_64-windows
              mv cgt-tools-v${version}-x86_64-windows.zip $out
            '';

          cgt-tools = mkCgtTools {
            inherit pkgs;
          };
        };

        devShells.default = pkgs.mkShell {
          shellHook = ''
            ${config.pre-commit.installationScript}
            PATH=$PATH:$(pwd)/target/release
          '';

          hardeningDisable = ["fortify"];

          nativeBuildInputs = [
            pkgs.${pythonToolchain}
            pkgs.${pythonToolchain}.pkgs.pip
            pkgs.alejandra
            pkgs.cargo-expand
            pkgs.cargo-flamegraph
            pkgs.cargo-leptos
            pkgs.cargo-machete
            pkgs.cargo-modules
            pkgs.cargo-nextest
            pkgs.cargo-tarpaulin
            pkgs.cargo-udeps
            pkgs.fd
            pkgs.graphviz
            pkgs.hyperfine
            pkgs.maturin
            pkgs.texlive.combined.scheme-full
            pkgs.trunk

            pkgs.pkg-config
            pkgs.SDL2

            pkgs.valgrind
            pkgs.kdePackages.kcachegrind

            rustToolchain
            # pkgs.wineWow64Packages.unstableFull
          ];
        };
        formatter = pkgs.alejandra;
      };
    });
}
