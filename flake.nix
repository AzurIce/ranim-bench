{
  description = "ranim";

  # nixConfig = {
  #   extra-substituters = [
  #     "https://mirrors.ustc.edu.cn/nix-channels/store"
  #   ];
  #   trusted-substituters = [
  #     "https://mirrors.ustc.edu.cn/nix-channels/store"
  #   ];
  # };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    ciallo.url = "github:azurice/ciallo";
  };

  outputs = { nixpkgs, crane, rust-overlay, flake-utils, ciallo, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        inherit (pkgs) lib;
        craneLib = (crane.mkLib pkgs).overrideToolchain (p:
          p.rust-bin.nightly.latest.default.override {
            targets = [ "wasm32-unknown-unknown" ];
            extensions = [ "rust-src" ];
          });
      in {
        devShells.default = craneLib.devShell {
          packages = (with pkgs; [ samply cargo-criterion ])
            ++ [ ciallo.packages.${system}.default ];
        };
      });
}
