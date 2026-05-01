{
  nodejs_24,
  buildNpmPackage,
  fira,
  jetbrains-mono,
  fira-mono,
  noto-fonts,
  fetchurl,
  name,
  sqlite,
  bash,
  node-gyp,
  pkg-config,
  vips,
  python3,
  gcc,
  vars ? "",
}:
let
  npmDepsHash = "sha256-6cKyzXrjiuLB/eI61nPgJ6VmjsksClkrJOHFdN/WCSI=";
  keys = fetchurl {
    url = "https://github.com/jdonszelmann.keys";
    sha256 = "sha256-zKL99PzMQ74pn0V2IwdFRgBQJdHnyDDGCDzMMBeSV54=";
  };
in
(buildNpmPackage {
  version = "22-05-2024";
  src = ../.;
  nativeBuildInputs = [
    node-gyp
    pkg-config
    python3
    gcc
  ];
  buildInputs = [
    nodejs_24
    sqlite
    vips
  ];

  npmFlags = [
    "--build-from-source"
    "--sqlite=${sqlite.dev}"
  ];
  inherit npmDepsHash name;
  configurePhase = ''
    mkdir -p ./public/fonts
    mkdir -p ./src/components

    ln -sf ${fira}/share/fonts/opentype/* ./public/fonts/
    ln -sf ${jetbrains-mono}/share/fonts/truetype/* ./public/fonts/
    ln -sf ${fira-mono}/share/fonts/opentype/* ./public/fonts/
    ln -sf ${noto-fonts}/share/fonts/noto/* ./public/fonts/

    cat > Keys.astro <<EOF
    ---
    import "../../public/style/keys.css"
    ---
    <ul class="keys">
    EOF
    cat ${keys} | xargs -I{} echo "<li>{}</li>" >> Keys.astro
    echo "</ul>" >> Keys.astro
    mv Keys.astro ./src/components/Keys.astro
  '';
  preBuild = ''
    ${vars}
    export npm_config_nodedir=${nodejs_24}
    export npm_config_sqlite=${sqlite.dev}
    export SHARP_FORCE_GLOBAL_LIBVIPS=1
  '';
  installPhase = ''
    mkdir -p $out/{homepage,bin,migrations}

    cp -pr migrations/* $out/migrations
    cp -pr dist/* $out/homepage
    cp -pr package.json $out/homepage
    cp -pr package-lock.json $out/homepage
    cp -pr node_modules $out/homepage

    cat > $out/bin/run <<EOF
      #!${bash}/bin/bash

      echo "migrations in $out/migrations"
      export CLIENT_DIR=$out/homepage/client

      cd $out/homepage
      exec ${nodejs_24}/bin/node server/entry.mjs
    EOF
    chmod +x $out/bin/run
  '';
})
