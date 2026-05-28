{
  rustChannelOf,
  naersk,
  callPackage,
  pkg-config,
  llvmPackages_latest,
  clang,
  lib,
  openssl_3,
}:
let
  toolchain =
    (rustChannelOf {
      rustToolchain = ../rust-toolchain.toml;
      sha256 = "sha256-+9FmLhAOezBZCOziO0Qct1NOrfpjNsXxc/8I0c7BdKE=";
    }).rust;

  naersk' = callPackage naersk {
    cargo = toolchain;
    rustc = toolchain;
  };
in
naersk'.buildPackage {
  src = ../.;
  buildInputs = [
    pkg-config
    clang
    llvmPackages_latest.bintools
    openssl_3
  ];
  LIBCLANG_PATH = lib.makeLibraryPath [
    llvmPackages_latest.libclang.lib
  ];
  PKG_CONFIG_PATH = "${openssl_3.dev}/lib/pkgconfig";
}
