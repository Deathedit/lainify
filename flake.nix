{
  description = "lainify — capture card viewer (Rust + Qt/QML + GStreamer)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forAllSystems (pkgs: {
        default = import ./shell.nix { inherit pkgs; };
      });

      packages = forAllSystems (pkgs: {
        default = pkgs.callPackage ./package.nix { };
      });

      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}
