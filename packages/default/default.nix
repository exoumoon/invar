{ inputs, pkgs, ... }:

let
    toolchain = inputs.fenix.packages.${pkgs.system}.fromToolchainFile {
        file = ../../rust-toolchain.toml;
        sha256 = "sha256-W40JpXO37SmKRpNDcDHUZ7nslk7A8SP0ja2BEnymCps=";
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
