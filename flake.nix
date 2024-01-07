{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        node-modules = pkgs.mkYarnPackage {
          name = "node-modules";
          src = ./.;
        };

        nativeBuildInputs = with pkgs; [ ];
        buildInputs = with pkgs; [ yarn node-modules ];

        dev = with pkgs;
          writeScriptBin "dev" ''
            yarn run start
          '';
      in
      with pkgs; rec {
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
              ${pkgs.yarn}/bin/yarn run build
            '';
            installPhase = ''
              mkdir $out
              cp -r dist $out
            '';
          };
          default = website;
        };
        devShells.default = mkShell {
          buildInputs = buildInputs ++ [
            dev
          ];
          inherit nativeBuildInputs;
          packages = with pkgs; [ lychee ];
        };
      });
}
