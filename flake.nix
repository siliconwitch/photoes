{
  description = "Photoes, a fast, minimal, AI-first photos app for Linux";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      forEachSystem =
        output:
        nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (
          system: output nixpkgs.legacyPackages.${system}
        );

      # gdk-pixbuf reads a single loaders.cache, and nixpkgs ships one per package.
      # This one lists the stock loaders plus HEIC.
      pixbufLoaders =
        pkgs:
        pkgs.runCommand "photoes-pixbuf-loaders.cache" { } ''
          ${pkgs.gdk-pixbuf.dev}/bin/gdk-pixbuf-query-loaders \
            ${pkgs.gdk-pixbuf.out}/lib/gdk-pixbuf-2.0/2.10.0/loaders/*.so \
            ${pkgs.librsvg}/lib/gdk-pixbuf-2.0/2.10.0/loaders/*.so \
            ${pkgs.libheif.lib}/lib/gdk-pixbuf-2.0/2.10.0/loaders/*.so \
            > $out
        '';
    in
    {
      packages = forEachSystem (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "photoes";
          version = (nixpkgs.lib.importTOML ./Cargo.toml).package.version;

          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs; [
            pkg-config
            wrapGAppsHook4
          ];

          buildInputs = with pkgs; [
            gtk4
            libadwaita
            libshumate
            libsoup_3
            glib-networking
            libjxl
            gst_all_1.gstreamer
            gst_all_1.gst-plugins-base
            gst_all_1.gst-plugins-good
            gst_all_1.gst-plugins-bad
          ];

          preFixup = ''
            gappsWrapperArgs+=(
              --set GDK_PIXBUF_MODULE_FILE ${pixbufLoaders pkgs}
              --prefix XDG_DATA_DIRS : ${pkgs.adwaita-icon-theme}/share
            )
          '';

          meta = {
            description = "Fast, minimal, AI-first photos app for Linux";
            homepage = "https://github.com/siliconwitch/photoes";
            license = nixpkgs.lib.licenses.gpl3Plus;
            mainProgram = "photoes";
            platforms = nixpkgs.lib.platforms.linux;
          };
        };
      });

      devShells = forEachSystem (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.default ];

          packages = with pkgs; [
            clippy
            rustfmt
            rust-analyzer
          ];

          # wrapGAppsHook4 sets these for the installed binary. `cargo run` needs them from the shell.
          shellHook = ''
            export GDK_PIXBUF_MODULE_FILE=${pixbufLoaders pkgs}
            export XDG_DATA_DIRS=$GSETTINGS_SCHEMAS_PATH:${pkgs.adwaita-icon-theme}/share:$XDG_DATA_DIRS
            export GIO_EXTRA_MODULES=${pkgs.glib-networking}/lib/gio/modules
          '';
        };
      });
    };
}
