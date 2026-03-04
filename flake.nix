{
  description = "Interactive trainer for learning Helix editor keybindings";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    helix-src = {
      url = "github:helix-editor/helix/25.07.1";
      flake = false;
    };
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];

      perSystem = {
        self',
        pkgs,
        lib,
        system,
        ...
      }: let
        rustToolchain = pkgs.rust-bin.stable."1.89.0".default;
        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

        nativeBuildInputs = with pkgs; [
          pkg-config
          rustToolchain
        ];

        buildInputs = with pkgs;
          [
            oniguruma # syntect regex-onig
          ]
          ++ lib.optionals stdenv.isLinux [
            alsa-lib # rodio audio playback
          ]
          ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.AudioUnit
            darwin.apple_sdk.frameworks.CoreAudio
            darwin.apple_sdk.frameworks.CoreFoundation
          ];

        # Patch the vendored deps to include languages.toml that
        # helix-loader expects via include_bytes!("../../languages.toml")
        vendorDir = craneLib.vendorCargoDeps {src = ./.;};
        patchedVendorDir = pkgs.runCommand "patched-vendor-deps" {} ''
          cp -rL ${vendorDir} $out
          chmod -R +w $out
          for loader_dir in $out/*/helix-loader-*; do
            if [ -d "$loader_dir" ]; then
              cp ${inputs.helix-src}/languages.toml "$(dirname "$loader_dir")/languages.toml"
            fi
          done
          substituteInPlace $out/config.toml \
            --replace-fail "${vendorDir}" "$out"
        '';

        src = let
          # Start with crane's standard Rust source filter
          cargoFilter = craneLib.filterCargoSources;
          # Also include non-Rust assets embedded via include_bytes!/include_str!
          assetsFilter = path: _type: let
            relPath = lib.removePrefix (toString ./. + "/") (toString path);
          in
            lib.hasPrefix "assets/" relPath
            || lib.hasPrefix "quests/" relPath
            || lib.hasPrefix "scenarios/" relPath
            || lib.hasPrefix "locales/" relPath;
        in
          lib.cleanSourceWith {
            src = ./.;
            filter = path: type: (cargoFilter path type) || (assetsFilter path type);
          };

        commonArgs = {
          inherit src;
          strictDeps = true;
          cargoVendorDir = patchedVendorDir;
          inherit nativeBuildInputs buildInputs;
          RUSTONIG_SYSTEM_LIBONIG = "1";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      in {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [(import inputs.rust-overlay)];
        };

        packages = {
          helix-trainer = craneLib.buildPackage (commonArgs
            // {
              inherit cargoArtifacts;
              # Some tests require HOME/filesystem access unavailable in the sandbox
              doCheck = false;
              meta = {
                description = "Interactive trainer for learning Helix editor keybindings";
                homepage = "https://github.com/bug-ops/helix-trainer";
                license = lib.licenses.mit;
                mainProgram = "helix-trainer";
              };
            });
          default = self'.packages.helix-trainer;
        };

        checks = {
          inherit (self'.packages) helix-trainer;

          clippy = craneLib.cargoClippy (commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            });

          fmt = craneLib.cargoFmt {
            inherit src;
          };
        };

        devShells.default = craneLib.devShell {
          checks = self'.checks;

          packages = with pkgs; [
            cargo-watch
            cargo-deny
          ];

          inherit buildInputs;
          RUSTONIG_SYSTEM_LIBONIG = "1";
        };
      };
    };
}
