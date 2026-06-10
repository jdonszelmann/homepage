{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    # for building rust packages
    naersk.url = "github:nix-community/naersk";
    # for eary pre-built toolchains
    nixpkgs-mozilla = {
      url = "github:mozilla/nixpkgs-mozilla";
      flake = false;
    };

    wild = {
      url = "github:wild-linker/wild";
      flake = false;
    };
  };
  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      nixpkgs-mozilla,
      naersk,
      wild,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (import nixpkgs-mozilla)
            (import wild)
          ];
        };

        toolchain = (
          (pkgs.rustChannelOf {
            rustToolchain = ./rust-toolchain.toml;
            sha256 = "sha256-mvUGEOHYJpn3ikC5hckneuGixaC+yGrkMM/liDIDgoU=";
          }).rust.override
            {
              extensions = [
                "rust-src"
              ];
            }
        );

        dev =
          with pkgs;
          writeScriptBin "dev" ''
            npm run start
          '';
        prefetch =
          with pkgs;
          writeScriptBin "prefetch" ''
            nix run nixpkgs#prefetch-npm-deps package-lock.json
          '';

        db_name = "homepage";

        wildStdenv = pkgs.useWildLinker pkgs.stdenv;
      in
      rec {
        packages = rec {
          website = pkgs.callPackage ./packages/homepage.nix { name = "homepage"; };
          website-gay = pkgs.callPackage ./packages/homepage.nix {
            vars = "export GAY=1";
            name = "homepage-gay";
          };
          default = website;

          website-rust = pkgs.callPackage ./packages/homepage-rust.nix {
            inherit naersk toolchain;
          };
        };
        devShells.default =
          with pkgs;
          mkShell.override { stdenv = wildStdenv; } {
            nativeBuildInputs = [ openssl ];
            buildInputs = [
              nodejs_24
              dev
              prefetch
              yarn

              openssl

              pkg-config
              ffmpeg
              clang
              llvmPackages_latest.bintools
              toolchain
            ];
            packages = [
              lychee

              (writeShellScriptBin "watch" ''
                yarn run dev --host '0.0.0.0:8000'
              '')

              (writeShellScriptBin "prep" ''
                cargo sqlx prepare
              '')

              gdb

              (postgresql.withPackages (postgresqlPackages: [ postgresqlPackages.pgtap ]))
            ];

            env = rec {
              DATABASE_URL = "postgres://postgres@${HOMEPAGE_DB_HOST}";

              HOMEPAGE_DB_USER = "postgres";
              HOMEPAGE_DB_HOST = "localhost";
              HOMEPAGE_DB_NAME = db_name;
              # note: not the same as used in production
              HOMEPAGE_CLIENT_SECRET = "35wTlOjLXEobV3lb3qaqHY018cFY5sO3";
              HOMEPAGE_CLIENT_ID = "1db518fb-6ba9-4f64-aee5-0ac3e97f358a";
              HOMEPAGE_AUTH_SERVER = "https://auth.donsz.nl";
              HOMEPAGE_BASE_URL = "http://localhost:3000";

              DATABASE_LOCATION = "./homepage.db";
              # note: not the same as used in production
              BETTER_AUTH_SECRET = "2/Uv6lUd5kNzjpgyoU9miAMuJEqLc4tOZhHS/LV4QGg=";
              BETTER_AUTH_URL = "http://localhost:4321";

            };

            shellHook = packages.website.configurePhase + ''
              export LIBCLANG_PATH="${lib.makeLibraryPath [ llvmPackages_latest.libclang.lib ]}"
              export LD_LIBRARY_PATH="'$LD_LIBRARY_PATH:${
                lib.makeLibraryPath [
                  openssl
                ]
              }"
              PKG_CONFIG_PATH="${openssl.dev}/lib/pkgconfig";
            '';
          };
      }
    );
}
