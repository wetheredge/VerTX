{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    flake-utils.url = "github:numtide/flake-utils";

    wrun.url = "github:wetheredge/wrun";
    wrun.inputs.nixpkgs.follows = "nixpkgs";
    wrun.inputs.flake-utils.follows = "flake-utils";

    esp-rs-nix.url = "github:leighleighleigh/esp-rs-nix";
    esp-rs-nix.inputs.nixpkgs.follows = "nixpkgs";
    esp-rs-nix.inputs.flake-utils.follows = "flake-utils";
  };

  outputs = inputs: inputs.flake-utils.lib.eachDefaultSystem (system: let
    pkgs = import inputs.nixpkgs {inherit system;};
    inherit (pkgs) lib;

    devPackages = with pkgs; [
      actionlint
      binaryen # wasm-opt
      biome
      bun
      cargo-nextest
      cargo-shear
      cargo-sort
      dprint
      probe-rs-tools
      rust-analyzer
      typescript-go
      typos
      wasm-bindgen-cli

      inputs.wrun.packages.${system}.default
      inputs.esp-rs-nix.package.${system}.esp-rs
    ];

    tsgoNix2Npm = v: lib.pipe v [
      (lib.removePrefix "0-unstable-")
      (lib.replaceString "-" "")
      (date: "7.0.0-dev.${date}.1")
    ];
    xtensaGcc = pkgs.callPackage "${inputs.esp-rs-nix}/esp-rs/xtensa-gcc.nix" {};
    versions = lib.pipe [devPackages xtensaGcc] [
      lib.flatten
      (lib.map ({name, pname ? name, version, ...}:
        if pname == "typescript-go" then {name = "@typescript/native-preview"; value = tsgoNix2Npm version;}
        else if pname == "esp-rs" then {name = "rust"; value = version;}
        else if pname == "esp-xtensa-gcc" then {name = "gcc"; value = version;}
        else {name = pname; value = version;}
      ))
      lib.listToAttrs
    ];
  in {
    devShells.default = pkgs.mkShell {
      packages = devPackages;
      shellHook = ''
        wrun setup:flake
      '';
    };

    inherit versions;
  });
}
