{
  lib,
  mkEffect,
  cargo,
  cargoSetupHook,
}:
mkEffect {
  buildInputs = [cargoSetupHook];
  inputs = [cargo];
  secretsMap = {"cargo" = "cratesIoToken";};

  effectScript = ''
    cargo publish --target-dir "$(mktemp -d)" -p cgt_derive
    cargo publish --target-dir "$(mktemp -d)" -p cgt
  '';
}
