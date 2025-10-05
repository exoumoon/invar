{ inputs, pkgs, ... }:

let
    toolchain = inputs.fenix.packages.${pkgs.system}.fromToolchainFile {
        file = ../../rust-toolchain.toml;
        sha256 = "sha256-SIPrdZidk0qhpUlQU6Ya/Cy/E5Dv8GHROdoh5XaFm6I=";
    };

    naersk' = pkgs.callPackage inputs.naersk {
        cargo = toolchain;
        rustc = toolchain;
    };
in

naersk'.buildPackage { src = ../..; }
