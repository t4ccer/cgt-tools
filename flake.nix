{
  description = "cgrs";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    pre-commit-hooks-nix = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.nixpkgs-stable.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
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
            "wasm32-unknown-emscripten"
          ];
        };

        pythonToolchain = "python311";
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
            rustfmt.enable = true;
            # clippy.enable = true;
          };
          tools = {
            rustfmt = lib.mkForce rustToolchain;
            clippy = lib.mkForce rustToolchain;
          };
        };

        devShells.default = pkgs.mkShell {
          shellHook = ''
            ${config.pre-commit.installationScript}
            PATH=$PATH:$(pwd)/target/release
          '';

          # env = {
          #   LD_LIBRARY_PATH = "${pkgs.stdenv.cc.cc.lib}/lib";
          # };

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
            pkgs.cargo-semver-checks
            pkgs.cargo-tarpaulin
            pkgs.cargo-udeps
            pkgs.fd
            pkgs.graphviz
            pkgs.heaptrack
            pkgs.hyperfine
            pkgs.maturin
            pkgs.sage
            pkgs.texlive.combined.scheme-full
            pkgs.trunk

            pkgs.wayland-scanner
            pkgs.pkg-config
            pkgs.cmake
            pkgs.xorg.libX11
            pkgs.xorg.libXrandr
            pkgs.xorg.libXinerama
            pkgs.xorg.libXcursor
            pkgs.mesa
            pkgs.xorg.libXi
            pkgs.clang
            pkgs.glxinfo
            pkgs.glfw3
            pkgs.wayland
            pkgs.libxkbcommon

            rustToolchain
          ];

          env.LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          env.LD_LIBRARY_PATH = lib.makeLibraryPath [
            pkgs.libGL
            pkgs.xorg.libXrandr
            pkgs.xorg.libXinerama
            pkgs.xorg.libXcursor
            pkgs.xorg.libXi
          ];
        };
        formatter = pkgs.alejandra;
      };
    };
}
