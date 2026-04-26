{ pkgs, ... }: let

invarAlias = pkgs.writeShellScriptBin "invar" ''
    cargo run -q -- $@
'';

in pkgs.mkShell {
    nativeBuildInputs = with pkgs; [ pkg-config invarAlias ];
    buildInputs = with pkgs; [ openssl ];
}
