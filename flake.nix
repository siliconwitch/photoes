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
            libjxl
            gst_all_1.gstreamer
            gst_all_1.gst-plugins-base
            gst_all_1.gst-plugins-good
            gst_all_1.gst-plugins-bad
          ];

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
        };
      });
    };
}
