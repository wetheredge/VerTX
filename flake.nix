{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    flake-utils.url = "github:numtide/flake-utils";

    wrun.url = "github:wetheredge/wrun";
    wrun.inputs.nixpkgs.follows = "nixpkgs";
    wrun.inputs.flake-utils.follows = "flake-utils";

    galock.url = "git+https://tangled.org/wetheredge.com/galock";
    galock.inputs.nixpkgs.follows = "nixpkgs";
    galock.inputs.flake-utils.follows = "flake-utils";

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
      cargo-nextest
      cargo-shear
      cargo-sort
      dprint
      nodejs-slim_latest
      pnpm
      probe-rs-tools
      rust-analyzer
      typescript-language-server
      typos
      wasm-bindgen-cli

      inputs.wrun.packages.${system}.default
      inputs.galock.packages.${system}.default
      inputs.esp-rs-nix.package.${system}.esp-rs
    ];

    xtensaGcc = pkgs.callPackage "${inputs.esp-rs-nix}/esp-rs/xtensa-gcc.nix" {};
    versions = lib.pipe [devPackages xtensaGcc] [
      lib.flatten
      (lib.map ({name, pname ? name, version, ...}:
        if pname == "nodejs-slim" then {name = "nodejs"; value = version;}
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
