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
  };
  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      nixpkgs-mozilla,
      naersk,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (import nixpkgs-mozilla)

          ];
        };

        toolchain = (
          (pkgs.rustChannelOf {
            rustToolchain = ./rust-toolchain.toml;
            sha256 = "sha256-gh/xTkxKHL4eiRXzWv8KP7vfjSk61Iq48x47BEDFgfk=";
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
          mkShell {
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
                cargo sqlx prepare --database-url=postgres://postgres@localhost
              '')

              gdb

              (postgresql.withPackages (postgresqlPackages: [ postgresqlPackages.pgtap ]))
            ];

            env = {
              PGDATA = "./.pgdata";

              HOMEPAGE_DB_HOST = "localhost";
              HOMEPAGE_DB_NAME = db_name;

              DATABASE_LOCATION = "./homepage.db";
              # note: not used in production
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
