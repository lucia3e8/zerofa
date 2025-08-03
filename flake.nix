{
  description = "Zero-FA - 2FA code email monitor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default;
        
        buildInputs = with pkgs; [
          openssl
          pkg-config
        ];
        
        nativeBuildInputs = with pkgs; [
          rustToolchain
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;
          
          shellHook = ''
            echo "Zero-FA development environment"
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "zerofa";
          version = "0.1.0";
          src = ./.;
          
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          
          inherit buildInputs nativeBuildInputs;
        };
        
        packages.static = let
          target = "x86_64-unknown-linux-musl";
          rustWithTarget = pkgs.rust-bin.stable.latest.default.override {
            targets = [ target ];
          };
        in pkgs.rustPlatform.buildRustPackage {
          pname = "zerofa-static";
          version = "0.1.0";
          src = ./.;
          
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          
          buildInputs = with pkgs.pkgsStatic; [
            openssl
          ];
          
          nativeBuildInputs = with pkgs; [
            pkg-config
            rustWithTarget
          ];
          
          CARGO_BUILD_TARGET = target;
          OPENSSL_STATIC = "1";
          OPENSSL_LIB_DIR = "${pkgs.pkgsStatic.openssl.out}/lib";
          OPENSSL_INCLUDE_DIR = "${pkgs.pkgsStatic.openssl.dev}/include";
          
          # Force static linking
          CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
        };
      });
}