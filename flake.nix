{
  description = "A virtual piano for the terminal, written in Rust";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      forAllSystems = f:
        nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ]
          (system: f (import nixpkgs { inherit system; }) system);
      manifest = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
      nativeDeps = pkgs: {
        nativeBuildInputs = [ pkgs.pkg-config ]
          ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.mold ];
        buildInputs = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.alsa-lib ];
      } // pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
        RUSTFLAGS = "-C link-arg=-B${pkgs.mold}/bin -C link-arg=-fuse-ld=mold";
      };
    in {
      devShells = forAllSystems (pkgs: _: {
        default = pkgs.mkShell (nativeDeps pkgs // {
          packages = [ pkgs.cargo pkgs.rustc ];
        });
      });
      packages = forAllSystems (pkgs: _: {
        default = pkgs.rustPlatform.buildRustPackage (nativeDeps pkgs // {
          pname = manifest.name;
          inherit (manifest) version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          doCheck = true;
          meta = with pkgs.lib; {
            description = "A virtual piano for the terminal";
            license = licenses.gpl3Only;
            mainProgram = manifest.name;
          };
        });
      });
      checks = forAllSystems (_: system: {
        build-and-test = self.packages.${system}.default;
      });
      apps = forAllSystems (pkgs: system: {
        default = {
          type = "app";
          program = pkgs.lib.getExe self.packages.${system}.default;
        };
      });
    };
}
