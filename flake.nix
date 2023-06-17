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
          buildInputs = with pkgs; [
            rustc
            rust-analyzer
            clippy
            cargo
            lldb_9
            sccache
            mold
            clang
            eww-wayland
            nushell
            jc
          ];
        pkgs = nixpkgs.legacyPackages.${system};
        naersk' = pkgs.callPackage naersk { };
        eww-utils =  naersk'.buildPackage {
          src = ./.;
          nativeBuildInputs = with pkgs; [ protobuf ];
          buildInputs = with pkgs; buildInputs ++ [ cargo rustc gcc cmake glibc stdenv.cc bash];
        };
      in {

        defaultPackage = pkgs.writeScript "eww"
        ''
        export PATH=$PATH:${pkgs.jq}/bin/:${pkgs.nushell}/bin/
        export EWW_UTILS=${eww-utils}/bin/eww-utils
        ${pkgs.eww-wayland}/bin/eww "$@"
        '';
        
        devShells.default = pkgs.mkShell {
        inherit buildInputs;
        };
      });
}

