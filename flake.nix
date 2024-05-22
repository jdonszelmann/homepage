{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        npmDepsHash = "sha256-WkGCXpmxuMfzZMG51EQrE7r46+lZ1WDVgLgGUtSlX6s=";

        nativeBuildInputs = with pkgs; [ ];
        buildInputs = with pkgs; [ nodejs ];

        dev = with pkgs;
          writeScriptBin "dev" ''
            npm run start
          '';
        prefetch = with pkgs;
          writeScriptBin "prefetch" ''
            nix run nixpkgs#prefetch-npm-deps package-lock.json
          '';
      in with pkgs; rec {
        packages = rec {
          inherit node-modules;
          website = pkgs.buildNpmPackage {
            name = "homepage";
            version = "22-05-2024";
            src = ./.;
            inherit nativeBuildInputs buildInputs npmDepsHash;
            buildPhase = ''
              ${pkgs.nodejs}/bin/npm run build
            '';
            installPhase = ''
              cp -pr dist $out/
            '';
          };
          default = website;
        };
        devShells.default = mkShell {
          buildInputs = buildInputs ++ [ dev prefetch ];
          inherit nativeBuildInputs;
          packages = with pkgs; [ lychee ];
        };
      });
}
