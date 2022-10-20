{
  description = "Control your Linux server from Discord";
  
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }: flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
      package = pkgs.rustPlatform.buildRustPackage {
        pname = "systemctl-bot";
        version = "0.4.2";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
      };
    in {
      packages.systemctl-bot = package;
      packages.default = package;
    });

}
