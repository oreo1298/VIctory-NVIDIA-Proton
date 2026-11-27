#!/usr/bin/env bash
# Builds the Windows victory.exe on Linux, to run under Proton or Wine (docs/linux-proton.md). This is the
# same Windows program the PowerShell launchers build, cross-compiled; it is not a native Linux build.
#
#   scripts/build-linux.sh                 release build: target/x86_64-pc-windows-msvc/release/victory.exe
#   scripts/build-linux.sh diag            the diagnostics build (target/x86_64-pc-windows-msvc/diag/)
#   scripts/build-linux.sh release --gnu   experimental: x86_64-pc-windows-gnu with mingw-w64 (see the doc)
#   scripts/build-linux.sh -- <args>       anything after -- goes to cargo (e.g. -- -p victory-core)
#
# Needs rustup (the toolchain is pinned in rust-toolchain.toml), cargo-xwin (cargo install cargo-xwin --locked),
# clang and lld (lld-link). The first build downloads Microsoft's CRT and Windows SDK libraries, under
# Microsoft's licence, into cargo-xwin's cache (~/.cache/cargo-xwin); later builds use that copy.
set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
build=release
gnu=0
extra=()
while (($#)); do
    case $1 in
        release | diag) build=$1 ;;
        --gnu) gnu=1 ;;
        -h | --help)
            sed -n '2,13p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        --)
            shift
            extra=("$@")
            break
            ;;
        *)
            echo "unknown option: $1 (see --help)" >&2
            exit 2
            ;;
    esac
    shift
done

need() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "$1 not found: $2" >&2
        exit 1
    fi
}
need cargo "install Rust with rustup (https://rustup.rs)"
need rustup "install Rust with rustup (https://rustup.rs); the toolchain is pinned in rust-toolchain.toml"

cd "$repo"
profile=(--release)
features=()
if [[ $build == diag ]]; then
    profile=(--profile diag)
    features=(--features diagnostics)
fi

if ((gnu)); then
    # Not the supported path: the MSVC target is. Kept for machines where cargo-xwin cannot fetch the CRT.
    target=x86_64-pc-windows-gnu
    need x86_64-w64-mingw32-gcc "install mingw-w64 (pacman -S mingw-w64-gcc, apt install mingw-w64)"
    rustup target add "$target" >/dev/null
    cargo build --locked "${profile[@]}" -p victory "${features[@]}" --target "$target" "${extra[@]}"
else
    target=x86_64-pc-windows-msvc
    need cargo-xwin "cargo install cargo-xwin --locked"
    need clang "install clang (pacman -S clang, apt install clang)"
    need lld-link "install lld (pacman -S lld, apt install lld)"
    cargo xwin build --locked "${profile[@]}" -p victory "${features[@]}" --target "$target" "${extra[@]}"
fi

out="$repo/target/$target/$build/victory.exe"
if [[ -f $out ]]; then
    echo "Built $out"
    echo "Start it with scripts/play-nvidia.sh --exe /path/to/game_scarlett_master.exe"
fi
