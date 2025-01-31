{
  description = "Control your Linux server from Discord";
  
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        package = pkgs.buildGoModule {
          name = "systemctl-bot";
          src = ./.;
          vendorHash = "sha256-HVQzM92rS8TgK+YXX0UinpvwRFEeFbeRVtp1jSLV14A=";
          doCheck = false;
        };
      in {
        packages.systemctl-bot = package;
        packages.default = package;
      }
    );

}