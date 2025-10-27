{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        dev = with pkgs;
          writeScriptBin "dev" ''
            npm run start
          '';
        prefetch = with pkgs;
          writeScriptBin "prefetch" ''
            nix run nixpkgs#prefetch-npm-deps package-lock.json
          '';
      in rec {
        packages = rec {
          website = pkgs.callPackage ./packages/homepage.nix {name = "homepage";};
          website-gay = pkgs.callPackage ./packages/homepage.nix {vars="export GAY=1"; name="homepage-gay";};
          default = website;
        };
        devShells.default = with pkgs; mkShell {
          buildInputs = [ nodejs dev prefetch yarn ];
          packages = [ lychee (pkgs.writeShellScriptBin "watch" ''
            yarn run dev --host '0.0.0.0:8000'
          '') ];
          shellHook = packages.website.configurePhase;
        };
      });
}
