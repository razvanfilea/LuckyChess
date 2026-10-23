#!/usr/bin/env bash
# Builds a single x86-64 Linux binary that runs the best engine build for the CPU.
#
# The engine is built once per CPU level as a static library. Each one is partially
# linked into a single object with every symbol made local except a uniquely named
# entry point, so the copies (each with its own std) can sit side by side in the
# launcher (chess_cli/src/universal.rs). The NNUE network is embedded only once.
#
# Usage: scripts/build_universal.sh [output path]

set -euo pipefail

OUTPUT=${1:+$(realpath -m "$1")}

cd "$(dirname "$0")/.."
TARGET=x86_64-unknown-linux-musl
OUT_DIR="$PWD/target/universal"
OUTPUT=${OUTPUT:-"$OUT_DIR/lucky_chess"}

# Must match SLICES in chess_cli/src/universal.rs
SLICES=(
    "x86_64_v2        -C target-cpu=x86-64-v2"
    "x86_64_v3_nopext -C target-cpu=x86-64-v3 -C target-feature=-bmi2"
    "x86_64_v3        -C target-cpu=x86-64-v3"
    "x86_64_v4        -C target-cpu=x86-64-v4"
)

mkdir -p "$OUT_DIR"
objects=()

for slice in "${SLICES[@]}"; do
    read -r name flags <<< "$slice"
    echo "==> Building $name"

    # Separate target dirs, since changing RUSTFLAGS invalidates the build
    CARGO_TARGET_DIR="$OUT_DIR/$name" RUSTFLAGS="$flags --cfg lucky_external_network" \
        cargo rustc --locked --profile dist --target "$TARGET" -p chess_cli --lib --crate-type staticlib

    object="$OUT_DIR/$name.o"
    ld -r --gc-sections -u lucky_slice_main -o "$object" "$OUT_DIR/$name/$TARGET/dist/libchess_cli.a"

    # The COMDAT group signature has to be unique too, otherwise the linker keeps
    # only one copy's and the others are left pointing at a discarded section
    objcopy \
        --redefine-sym "lucky_slice_main=lucky_main_$name" \
        --redefine-sym "DW.ref.rust_eh_personality=DW.ref.rust_eh_personality_$name" \
        --keep-global-symbol "lucky_main_$name" \
        "$object"
    objects+=("$object")
done

rm -f "$OUT_DIR/liblucky_slices.a"
ar rcs "$OUT_DIR/liblucky_slices.a" "${objects[@]}"

echo "==> Linking launcher"
# Overrides any target-cpu from the cargo config: the launcher must run on every x86-64 CPU
CARGO_TARGET_DIR="$OUT_DIR/launcher" RUSTFLAGS="-C target-cpu=x86-64" cargo rustc --locked --profile dist --target "$TARGET" \
    -p chess_cli --bin lucky_chess -- --cfg lucky_universal -L "$OUT_DIR" -l static=lucky_slices

cp "$OUT_DIR/launcher/$TARGET/dist/lucky_chess" "$OUTPUT"
echo "==> $OUTPUT"
