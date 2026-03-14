{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain

            # Build dependencies
            pkg-config

            # evdev / uinput (Linux input)
            linuxHeaders

            # egui dependencies (GPU rendering)
            libxkbcommon
            libGL
            wayland
            libx11
            libxcursor
            libxrandr
            libxi

            # Output backends
            xdotool
            wtype
          ];

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
            libxkbcommon
            libGL
            wayland
            libx11
            libxcursor
            libxrandr
            libxi
          ]);

          RUST_BACKTRACE = "1";

          shellHook = ''
            echo "klav dev shell ready — $(rustc --version)"
          '';
        };
      }
    );
}
