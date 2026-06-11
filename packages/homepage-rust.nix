{
  naersk,
  callPackage,
  pkg-config,
  llvmPackages_latest,
  clang,
  lib,
  openssl_3,
  fira,
  jetbrains-mono,
  fira-mono,
  noto-fonts,
  fetchurl,
}:
let
  naersk' = callPackage naersk { };

  keys = fetchurl {
    url = "https://github.com/jdonszelmann.keys";
    sha256 = "sha256-zKL99PzMQ74pn0V2IwdFRgBQJdHnyDDGCDzMMBeSV54=";
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

  preConfigure = ''
    mkdir -p ./public/fonts
    mkdir -p ./templates

    ln -sf ${fira}/share/fonts/opentype/* ./public/fonts/
    ln -sf ${jetbrains-mono}/share/fonts/truetype/* ./public/fonts/
    ln -sf ${fira-mono}/share/fonts/opentype/* ./public/fonts/
    ln -sf ${noto-fonts}/share/fonts/noto/* ./public/fonts/

    touch keys.html
    cat > keys.html <<EOF
    <ul class="keys">
    EOF
    cat ${keys} | xargs -I{} echo "<li>{}</li>" >> keys.html
    echo "</ul>" >> keys.html
    mv keys.html ./templates/keys.html
  '';

  preBuild = ''
    rm -rf .cargo
  '';

  postInstall = ''
    cp -r ./public $out
  '';
}
