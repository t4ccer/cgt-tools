{
  lib,
  mkEffect,
  cargo,
  stdenv,
  cargoSetupHook,
}: {src}:
mkEffect {
  buildInputs = [cargoSetupHook];
  inputs = [cargo stdenv.cc];
  secretsMap = {"cargo" = "cratesIoToken";};
  inherit src;

  effectScript = ''
    cargo publish --target-dir "$(mktemp -d)" -p cgt_derive
    cargo publish --target-dir "$(mktemp -d)" -p cgt
  '';
}
