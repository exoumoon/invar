{ pkgs, lib, ... }: let

invarAlias = pkgs.writeShellScriptBin "invar" ''
    cargo run -q -- $@
'';

in pkgs.mkShell rec {
    nativeBuildInputs = with pkgs; [ pkg-config invarAlias ];
    buildInputs = with pkgs; [ openssl ];
    NIX_LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
    LD_LIBRARY_PATH = NIX_LD_LIBRARY_PATH;
}
