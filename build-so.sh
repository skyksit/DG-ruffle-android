#!/bin/bash
#
# Build libruffle_android.so for the DGPlayer host app.
#
# This repository's Android APP is not shipped -- the deliverable is the four
# per-ABI `libruffle_android.so` files that a host app bundles as jniLibs.
# This script builds only those. It does not build or install an APK.
#
# Usage:
#   ./build-so.sh                          # build all ABIs into dist/jniLibs
#   ./build-so.sh --abi arm64-v8a          # build one ABI (repeatable)
#   ./build-so.sh --out <dir>              # write jniLibs tree elsewhere
#   ./build-so.sh --copy-to <dir>          # also copy into a host app's jniLibs
#   ./build-so.sh --lib-name <file.so>     # rename output (e.g. libruffle_android_v7.so)
#   ./build-so.sh --debug                  # cargo debug profile (keeps logging)
#
# Environment overrides:
#   ANDROID_NDK_HOME / ANDROID_NDK_ROOT    NDK location (else autodetected)
#   API_LEVEL                              defaults to 26, must match host minSdk
#   LIB_NAME                               same as --lib-name
#
# Renaming is safe: the library carries no SONAME, so it resolves by path
# (System.load) or by filename in jniLibs (System.loadLibrary). The host app
# picks a versioned core name, e.g. libruffle_android_v7.so.
#
set -euo pipefail

RED=$'\033[0;31m'; GREEN=$'\033[0;32m'; YELLOW=$'\033[1;33m'; BLUE=$'\033[0;34m'; NC=$'\033[0m'
step()  { printf '\n%s==> %s%s\n' "$BLUE" "$1" "$NC"; }
ok()    { printf '%s[ok]%s %s\n' "$GREEN" "$NC" "$1"; }
warn()  { printf '%s[warn]%s %s\n' "$YELLOW" "$NC" "$1"; }
die()   { printf '%s[error]%s %s\n' "$RED" "$NC" "$1" >&2; exit 1; }

# --- Paths are derived from this script's location, never hardcoded ------------
REPO_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
OUT_DIR="${REPO_DIR}/dist/jniLibs"
COPY_TO=""
PROFILE="release"
# cargo-ndk always emits libruffle_android.so; this is the name we ship it under.
LIB_NAME="${LIB_NAME:-libruffle_android.so}"
API_LEVEL="${API_LEVEL:-26}"
ABIS=()

while [ $# -gt 0 ]; do
    case "$1" in
        --abi)      [ $# -ge 2 ] || die "--abi needs a value"; ABIS+=("$2"); shift 2 ;;
        --out)      [ $# -ge 2 ] || die "--out needs a value"; OUT_DIR="$2"; shift 2 ;;
        --copy-to)  [ $# -ge 2 ] || die "--copy-to needs a value"; COPY_TO="$2"; shift 2 ;;
        --lib-name) [ $# -ge 2 ] || die "--lib-name needs a value"; LIB_NAME="$2"; shift 2 ;;
        --debug)    PROFILE="debug"; shift ;;
        --release)  PROFILE="release"; shift ;;
        -h|--help)  sed -n '3,25p' "${BASH_SOURCE[0]}"; exit 0 ;;
        *)          die "unknown argument: $1" ;;
    esac
done

# Default to the four ABIs the host app bundles.
if [ ${#ABIS[@]} -eq 0 ]; then
    ABIS=(arm64-v8a armeabi-v7a x86 x86_64)
fi

# --- Locate the NDK -----------------------------------------------------------
step "Locating Android NDK"

find_ndk() {
    # 1. Explicit environment.
    for candidate in "${ANDROID_NDK_HOME:-}" "${ANDROID_NDK_ROOT:-}"; do
        if [ -n "$candidate" ] && [ -d "$candidate" ]; then
            printf '%s' "$candidate"; return 0
        fi
    done

    # 2. sdk.dir from local.properties, then newest ndk/<version> under it.
    #    Java .properties escapes the drive colon as `D\:/...` -- unescape it.
    local sdk_dir=""
    if [ -f "${REPO_DIR}/local.properties" ]; then
        sdk_dir="$(sed -n 's/^[[:space:]]*sdk\.dir[[:space:]]*=[[:space:]]*//p' \
                   "${REPO_DIR}/local.properties" | tail -n 1 | tr -d '\r')"
        # `D\:/Android/Sdk` -> `D:/Android/Sdk`; strip the properties escapes.
        sdk_dir="${sdk_dir//\\/}"
    fi
    [ -n "$sdk_dir" ] || sdk_dir="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
    [ -n "$sdk_dir" ] || return 1

    local newest
    newest="$(ls -1d "${sdk_dir}/ndk"/*/ 2>/dev/null | sed 's#/$##' | sort -V | tail -n 1)"
    [ -n "$newest" ] && [ -d "$newest" ] || return 1
    printf '%s' "$newest"
}

ANDROID_NDK_HOME="$(find_ndk)" \
    || die "NDK not found. Set ANDROID_NDK_HOME, or set sdk.dir in local.properties with an ndk/ subdirectory installed."
export ANDROID_NDK_HOME
export ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"
ok "NDK: $ANDROID_NDK_HOME"

# --- Toolchain checks ---------------------------------------------------------
step "Checking Rust toolchain"

command -v cargo    >/dev/null 2>&1 || die "cargo not found. Install Rust: https://rustup.rs/"
command -v cargo-ndk >/dev/null 2>&1 \
    || die "cargo-ndk not found. Install it yourself: cargo install cargo-ndk"

# Map ABI -> Rust target triple so we can check only what we are about to build.
abi_to_triple() {
    case "$1" in
        arm64-v8a)   printf 'aarch64-linux-android' ;;
        armeabi-v7a) printf 'armv7-linux-androideabi' ;;
        x86)         printf 'i686-linux-android' ;;
        x86_64)      printf 'x86_64-linux-android' ;;
        *)           return 1 ;;
    esac
}

installed_targets="$(rustup target list --installed 2>/dev/null || true)"
missing=()
for abi in "${ABIS[@]}"; do
    triple="$(abi_to_triple "$abi")" || die "unknown ABI: $abi"
    printf '%s\n' "$installed_targets" | grep -qx "$triple" || missing+=("$triple")
done
if [ ${#missing[@]} -gt 0 ]; then
    die "Missing Rust targets: ${missing[*]}
Install them yourself:  rustup target add ${missing[*]}"
fi
ok "cargo, cargo-ndk, and targets for ${ABIS[*]} are present"

# --- Build --------------------------------------------------------------------
step "Building ${LIB_NAME} (${PROFILE}, API ${API_LEVEL})"

target_flags=()
for abi in "${ABIS[@]}"; do
    target_flags+=(-t "$abi")
done

cargo_args=(build)
[ "$PROFILE" = "release" ] && cargo_args+=(--release)

mkdir -p "$OUT_DIR"
warn "Building ${#ABIS[@]} ABI(s); a cold build takes several minutes."
( cd "$REPO_DIR" && cargo ndk "${target_flags[@]}" -P "$API_LEVEL" -o "$OUT_DIR" "${cargo_args[@]}" )

# --- Verify -------------------------------------------------------------------
step "Verifying output"

BUILT_NAME="libruffle_android.so"
SO_NAME="$LIB_NAME"

# cargo-ndk names the output after the crate; rename if a different one is asked for.
if [ "$SO_NAME" != "$BUILT_NAME" ]; then
    for abi in "${ABIS[@]}"; do
        if [ -f "${OUT_DIR}/${abi}/${BUILT_NAME}" ]; then
            mv -f "${OUT_DIR}/${abi}/${BUILT_NAME}" "${OUT_DIR}/${abi}/${SO_NAME}"
        fi
    done
    ok "renamed output to ${SO_NAME}"
fi

for abi in "${ABIS[@]}"; do
    so_path="${OUT_DIR}/${abi}/${SO_NAME}"
    [ -f "$so_path" ] || die "missing: ${so_path}"
    ok "${abi}: $(du -h "$so_path" | cut -f1)"
done

# The host app resolves these lazily, so a missing export is a crash at first
# call rather than a link error. Check them here instead.
EXPECTED_SYMBOLS=18
# Prefer the NDK's llvm-readelf: a stray GNU readelf (e.g. msys2) prints these
# names in a form the grep below does not match, which would look like a
# missing-symbol failure when the build is in fact fine.
readelf_bin=""
for candidate in "${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt"/*/bin/llvm-readelf* llvm-readelf readelf; do
    if command -v "$candidate" >/dev/null 2>&1; then readelf_bin="$candidate"; break; fi
done
if [ -n "$readelf_bin" ]; then
    first_abi="${ABIS[0]}"
    sym_count="$("$readelf_bin" --dyn-syms "${OUT_DIR}/${first_abi}/${SO_NAME}" 2>/dev/null \
                 | grep -c 'Java_rs_ruffle_PlayerActivity_' || true)"
    if [ "${sym_count:-0}" -eq "$EXPECTED_SYMBOLS" ]; then
        ok "${first_abi}: all ${EXPECTED_SYMBOLS} Java_rs_ruffle_PlayerActivity_* symbols exported"
    else
        warn "${first_abi}: expected ${EXPECTED_SYMBOLS} exported JNI symbols, found ${sym_count:-0}."
        warn "See the host integration contract in README.md before shipping this build."
    fi
else
    warn "llvm-readelf not found; skipped the JNI symbol check."
fi

# --- Optional copy into a host app -------------------------------------------
if [ -n "$COPY_TO" ]; then
    step "Copying into host app"
    [ -d "$COPY_TO" ] || die "--copy-to target does not exist: $COPY_TO"
    for abi in "${ABIS[@]}"; do
        mkdir -p "${COPY_TO}/${abi}"
        cp -f "${OUT_DIR}/${abi}/${SO_NAME}" "${COPY_TO}/${abi}/${SO_NAME}"
        ok "${abi} -> ${COPY_TO}/${abi}/${SO_NAME}"
    done
fi

step "Done"
printf 'jniLibs: %s\n' "$OUT_DIR"
if [ -z "$COPY_TO" ]; then
    printf 'Copy into a host app with: %s--copy-to <app>/src/main/jniLibs%s\n' "$YELLOW" "$NC"
fi
printf 'Host-side requirements: see "Host integration contract" in README.md\n'
