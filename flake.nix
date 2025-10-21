{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        npmDepsHash = "sha256-xhw+CutTTQzQCci+TUWifez0JofIYU+izySnlPfVBJE=";

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
        nerdfonts =
          pkgs.nerdfonts.override { fonts = [ "NerdFontsSymbolsOnly" ]; };
        build = vars: name: (pkgs.buildNpmPackage {
                  version = "22-05-2024";
                  src = ./.;
                  inherit nativeBuildInputs buildInputs npmDepsHash name;
                  configurePhase = ''
                    mkdir -p ./public/fonts
                    ln -sf ${nerdfonts}/share/fonts/truetype/NerdFonts/* ./public/fonts/
                    ln -sf ${pkgs.fira}/share/fonts/opentype/* ./public/fonts/
                    ln -sf ${pkgs.jetbrains-mono}/share/fonts/truetype/* ./public/fonts/
                    ln -sf ${pkgs.fira-mono}/share/fonts/opentype/* ./public/fonts/
                  '';
                  buildPhase = ''
                    ${vars}
                    ${pkgs.nodejs}/bin/npm run build
                  '';
                  installPhase = ''
                    cp -pr dist $out
                  '';
                });
      in rec {
        packages = rec {
          website = build "" "homepage";
          website-gay = build "export GAY=1" "homepage-gay";
          default = website;
        };
        devShells.default = with pkgs; mkShell {
          buildInputs = buildInputs ++ [ dev prefetch yarn ];
          inherit nativeBuildInputs;
          packages = with pkgs; [ lychee (pkgs.writeShellScriptBin "watch" ''
            yarn run dev --host '0.0.0.0:8000'
          '') ];
          shellHook = packages.website.configurePhase;
        };
      });
}
