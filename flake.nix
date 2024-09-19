{
  description = "Submerge combines subtitles from two files into one, with customizable position and color settings";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    # Provides helpers for Rust toolchains
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      # Systems supported
      allSystems = [
        "x86_64-linux" # 64-bit Intel/AMD Linux
        "aarch64-linux" # 64-bit ARM Linux
        "x86_64-darwin" # 64-bit Intel macOS
        "aarch64-darwin" # 64-bit ARM macOS
      ];

      # Helper to provide system-specific attributes
      forAllSystems = f: nixpkgs.lib.genAttrs allSystems (system: f {
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            # Provides Nixpkgs with a rust-bin attribute for building Rust toolchains
            rust-overlay.overlays.default
            # Uses the rust-bin attribute to select a Rust toolchain
            self.overlays.default
          ];
        };
      });
    in
    {
      overlays.default = final: prev: {
        # The Rust toolchain used for the package build
        rustToolchain = final.rust-bin.nightly.latest.default;
      };

      devShells = forAllSystems ({ pkgs } : {
        default = pkgs.mkShell {
          buildInputs = [
            pkgs.rustToolchain
            pkgs.rust-analyzer
            (pkgs.writeShellScriptBin "fmt-check" ''
              cargo fmt --manifest-path ./Cargo.toml --all -- --check
            '')
          ];
        };
      });

      packages = forAllSystems ({ pkgs }: {
        default =
          let
            manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
            rustPlatform = pkgs.makeRustPlatform {
              cargo = pkgs.rustToolchain;
              rustc = pkgs.rustToolchain;
            };
          in
          rustPlatform.buildRustPackage {
            name = manifest.name;
            version = manifest.version;
            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
          };
      });
    };
}
