{ pkgs, lib, ... }:

pkgs.mkShell rec {
    nativeBuildInputs = with pkgs; [ pkg-config ];
    buildInputs = with pkgs; [ openssl ];
    NIX_LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
}
