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

    pupgrade.url = "git+https://tangled.org/wetheredge.com/pupgrade";
    pupgrade.inputs.nixpkgs.follows = "nixpkgs";
    pupgrade.inputs.flake-utils.follows = "flake-utils";
    pupgrade.inputs.galock.follows = "galock";

    esp-rs-nix.url = "github:leighleighleigh/esp-rs-nix";
    esp-rs-nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs: inputs.flake-utils.lib.eachDefaultSystem (system: let
    pkgs = import inputs.nixpkgs {inherit system;};
    inherit (pkgs) lib;

    inherit (inputs.esp-rs-nix.packages.${system}) esp-rs esp-xtensa-gcc;

    # TODO: remove this once it lands in unstable
    wasm-bindgen-cli = pkgs.buildWasmBindgenCli rec {
      src = pkgs.fetchCrate {
        pname = "wasm-bindgen-cli";
        version = "0.2.105";
        hash = "sha256-zLPFFgnqAWq5R2KkaTGAYqVQswfBEYm9x3OPjx8DJRY";
      };

      cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
        inherit src;
        inherit (src) pname version;
        hash = "sha256-a2X9bzwnMWNt0fTf30qAiJ4noal/ET1jEtf5fBFj5OU=";
      };
    };

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
      rustup # unsure *how* it tells cargo where to find std,core,etc but it does
      typescript-language-server
      typos
      wasm-bindgen-cli

      inputs.wrun.packages.${system}.default
      inputs.galock.packages.${system}.default
      inputs.pupgrade.packages.${system}.default
      esp-rs
    ];

    versions = lib.pipe [devPackages esp-xtensa-gcc] [
      lib.flatten
      (lib.map ({name, pname ? name, version, ...}:
        if pname == "nodejs-slim" then {name = "nodejs"; value = version;}
        else if pname == "esp-rust-src" then {name = "rust"; value = version;}
        else if pname == "esp-xtensa-gcc" then {name = "gcc"; value = version;}
        else {name = pname; value = version;}
      ))
      lib.listToAttrs
    ];
  in {
    devShells.default = pkgs.mkShell {
      packages = devPackages;
      shellHook = ''
        export RUSTUP_TOOLCHAIN='${esp-rs}'

        wrun setup:flake
      '';
    };

    inherit versions;
  });
}
