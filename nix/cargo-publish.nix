{
  lib,
  mkEffect,
  cargo,
  cargoSetupHook,
}: {src}:
mkEffect {
  buildInputs = [cargoSetupHook];
  inputs = [cargo];
  secretsMap = {"cargo" = "cratesIoToken";};
  inherit src;

  effectScript = ''
    cargo publish --target-dir "$(mktemp -d)" -p cgt_derive
    cargo publish --target-dir "$(mktemp -d)" -p cgt
  '';
}
