{
  nodejs,
  buildNpmPackage,
  nerdfonts,
  fira,
  jetbrains-mono,
  fira-mono,
  fetchurl,
  name,
  vars ? "",
}:
let
  nerdfonts-symbols = nerdfonts.override { fonts = [ "NerdFontsSymbolsOnly" ]; };
  npmDepsHash = "sha256-xhw+CutTTQzQCci+TUWifez0JofIYU+izySnlPfVBJE=";
  keys = fetchurl {
    url = "https://github.com/jdonszelmann.keys";
    sha256 = "sha256:1sla88pmh16jg9zx7kacca96599j38b2c340hlkdvzgjpclys28c";
  };
in
(buildNpmPackage {
  version = "22-05-2024";
  src = ../.;
  nativeBuildInputs = [ ];
  buildInputs = [ nodejs ];
  inherit npmDepsHash name;
  configurePhase = ''
    mkdir -p ./public/fonts
    mkdir -p ./src/components

    ln -sf ${nerdfonts-symbols}/share/fonts/truetype/NerdFonts/* ./public/fonts/
    ln -sf ${fira}/share/fonts/opentype/* ./public/fonts/
    ln -sf ${jetbrains-mono}/share/fonts/truetype/* ./public/fonts/
    ln -sf ${fira-mono}/share/fonts/opentype/* ./public/fonts/

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
  buildPhase = ''
    ${vars}
    ${nodejs}/bin/npm run build
  '';
  installPhase = ''
    cp -pr dist $out
  '';
})
