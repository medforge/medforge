{
  inputs = {
    flake-schemas.url = "https://flakehub.com/f/DeterminateSystems/flake-schemas/*";
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/*";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, flake-schemas, nixpkgs, rust-overlay }:
    let
      supportedSystems = [ "aarch64-darwin" "x86_64-darwin" "x86_64-linux" "aarch64-linux" ];
      forEachSupportedSystem = f: nixpkgs.lib.genAttrs supportedSystems (system: f {
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
      });
    in {
      schemas = flake-schemas.schemas;

      devShells = forEachSupportedSystem ({ pkgs }: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            # Rust toolchain (latest stable, edition 2024)
            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" "clippy" "rustfmt" ];
            })

            # Python
            python312
            python312Packages.pytest
            python312Packages.pip

            # Build tooling
            maturin

            # Dev tools
            nixpkgs-fmt
          ];

          shellHook = ''
            echo "🔧 medforge dev shell"
            echo "  rust: $(rustc --version)"
            echo "  python: $(python3 --version)"
            echo "  maturin: $(maturin --version)"
          '';
        };
      });
    };
}
