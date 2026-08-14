{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "keyinject";
  version = "0.1.0";
  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Emit one key tap through Linux uinput";
    license = lib.licenses.mit;
    mainProgram = "keyinject";
  };
}
