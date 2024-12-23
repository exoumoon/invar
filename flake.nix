{
    description = "CLI management tool for modded Minecraft servers";

    inputs = {
        nixpkgs.url = "github:nixos/nixpkgs/nixos-24.11";
        naersk.url = "github:nix-community/naersk";
        fenix.url = "github:nix-community/fenix";
        snowfall-lib = {
            url = "github:mxxntype/snowfall";
            inputs.nixpkgs.follows = "nixpkgs";
        };

    };

    outputs = inputs: inputs.snowfall-lib.mkFlake {
        inherit inputs;
        src = ./.;
        overlays = with inputs; [ fenix.overlays.default ];
    };
}
