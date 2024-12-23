{ inputs, pkgs, ... }:

let
    toolchain = inputs.fenix.packages.${pkgs.system}.fromToolchainFile {
        file = ../../rust-toolchain.toml;
        sha256 = "sha256-xpStU6xQanJNSXnOU9AY7nz9Ycjlv0/eQkNHP1LSBoc=";
    };

    naersk' = pkgs.callPackage inputs.naersk {
        cargo = toolchain;
        rustc = toolchain;
    };
in

naersk'.buildPackage { src = ../..; }
