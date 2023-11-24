{
  description = "Devshell for esp32c3 dev";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        rust-toolchain = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [ "llvm-tools-preview" "rust-src" ];
          targets = [ "riscv32imc-unknown-none-elf" ];
        };

      in
      {
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rust-toolchain
            rust-analyzer
            cargo-show-asm
            cargo-espflash
            cargo-outdated
            taplo
            probe-rs
            pkg-config
            udev
          ];
        };
      }
    );
}
