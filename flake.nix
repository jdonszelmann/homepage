{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        node-modules = pkgs.buildNpmPackage {
          name = "node-modules";
          src = ./.;
          npmDepsHash = "sha256-yvESIlYLlyX8oOWzDmcqx0HNSKfTXQcEvyI4UfhrHZw=";
        };

        nativeBuildInputs = with pkgs; [ ];
        buildInputs = with pkgs; [ nodejs node-modules ];

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
          website = pkgs.stdenv.mkDerivation rec {
            pname = "homepage";
            version = "2024-01-07";
            src = self;
            inherit nativeBuildInputs buildInputs;
            buildPhase = ''
              ln -s ${node-modules}/libexec/homepage/node_modules node_modules
              export HOME=$TMPDIR
              ${pkgs.nodejs}/bin/npm run build
            '';
            installPhase = ''
              runHook preInstall
              cp -pr dist $out/
              runHook postInstall
            '';
          };
          default = website;
        };
        devShells.default = mkShell {
          buildInputs = buildInputs ++ [ dev ];
          inherit nativeBuildInputs;
          packages = with pkgs; [ lychee ];
        };
      });
}
