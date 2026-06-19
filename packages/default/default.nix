{ inputs, pkgs, ... }:

let
    toolchain = inputs.fenix.packages.${pkgs.system}.fromToolchainFile {
        file = ../../rust-toolchain.toml;
        sha256 = "sha256-1JnTD7VQWtPnJ21sckiOs/b8TNAr6J/qnamUIjQy0zU=";
    };

    naersk' = pkgs.callPackage inputs.naersk {
        cargo = toolchain;
        rustc = toolchain;
    };
in

naersk'.buildPackage {
    pname = "invar";
    src = ../..;
    nativeBuildInputs = with pkgs; [ autoPatchelfHook ];
    buildInputs = with pkgs; [ openssl.dev libgcc.lib ];
}
