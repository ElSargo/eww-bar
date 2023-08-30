{
  description = "Dev shell the project";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
  };
  outputs = { self, nixpkgs, flake-utils, naersk }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        devTools = with pkgs; [
          lldb_9
          sccache
          rust-analyzer
          clippy
        ];
        
        buildInputs = with pkgs; [
          cargo
          mold
        ];
        pkgs = nixpkgs.legacyPackages.${system};
        naersk' = pkgs.callPackage naersk { };
        eww-utils = naersk'.buildPackage {
          src = ./.;
          buildInputs = buildInputs;
        };
      in rec {

        overlays.default = (self: super: { eww-bar = packages.default ; });

        packages.default = pkgs.writeShellApplication {
          name = "eww";

          runtimeInputs = with pkgs; [ jc nushell ];

          text = ''
            export EWW_UTILS=${eww-utils}/bin/eww-utils
            ([ -d "$HOME/.config/eww" ] && ${pkgs.eww-wayland}/bin/eww "$@") || ${pkgs.eww-wayland}/bin/eww -c "${self}/" "$@"
          '';
        };

        devShells.default = pkgs.mkShell { buildInputs = buildInputs ++ devTools; };
      });
}

