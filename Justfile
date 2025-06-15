TARGET_BINARY := "./target/debug/invar"

# List out available recipes.
default:
    @just --list

# Generate shell completions for all supported shells.
generate-completions:
    cargo build
    mkdir -pv target/completions
    {{TARGET_BINARY}} completions --shell bash > target/completions/bash.sh
    {{TARGET_BINARY}} completions --shell elvish > target/completions/elvish.elv
    {{TARGET_BINARY}} completions --shell fish > target/completions/fish.fish
    {{TARGET_BINARY}} completions --shell power-shell > target/completions/powershell.bat
    {{TARGET_BINARY}} completions --shell zsh > target/completions/zsh.zsh
    {{TARGET_BINARY}} completions --shell nushell > target/completions/nushell.nu
