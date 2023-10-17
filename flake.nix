{
  description = "Control your Linux server from Discord";
  
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        package = pkgs.rustPlatform.buildRustPackage {
          pname = "systemctl-bot";
          version = "0.4.2";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
        };
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            (pkgs.rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" ];
            })
          ];
        };
        packages.systemctl-bot = package;
        packages.default = package;
      }
    );

}
