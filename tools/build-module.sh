#!/bin/bash
# Build one package's WebAssembly module and put it where its manifest says.
#   tools/build-module.sh MODULE/Cargo.toml DEST.wasm [TARGET_DIR]
# A module keeps the paths of the files it was built from, for its error
# messages; built as it comes, they'd be the builder's own folders (their home
# folder among them) in every copy of the package. So the repository's folder
# is written as "." and Cargo's as "cargo", with rustc's --remap-path-prefix
# (Cargo's trim-paths does this but isn't stable in Rust 1.98). The flags go
# to rustc through this script, as Cargo's compiler wrapper, not through
# RUSTFLAGS: Cargo hashes RUSTFLAGS into the module's symbols
# (rust-lang/cargo#6914), and the folders in them would make every folder's
# build a different file. This way the same source and the same Rust make the
# same module wherever it's built. tools/release-package.py refuses a module
# that still holds a folder of the builder's.
set -euo pipefail
if [ -n "${ECTOMATTER_REMAP_ROOT:-}" ]; then
  # (called by Cargo as the wrapper: rustc and its arguments follow)
  exec "$@" "--remap-path-prefix=$ECTOMATTER_REMAP_ROOT=." "--remap-path-prefix=$ECTOMATTER_REMAP_CARGO=cargo"
fi
# (a folder of builds only this script makes: Cargo doesn't count the wrapper
# when it decides a library is built already, so a build made without it
# elsewhere would be taken as it is)
manifest=$1 dest=$2 target=${3:-target/modules}
here=$(cd "$(dirname "$0")" && pwd)
cargo_home=${CARGO_HOME:-$HOME/.cargo}
export PATH="$cargo_home/bin:$PATH"
export ECTOMATTER_REMAP_ROOT=$(dirname "$here") ECTOMATTER_REMAP_CARGO=$cargo_home RUSTC_WRAPPER="$here/build-module.sh"
cargo build --quiet --release --target wasm32-wasip2 --manifest-path "$manifest" --target-dir "$target"
name=$(sed -n 's/^name = "\(.*\)"/\1/p' "$manifest" | head -1 | tr - _)
cp "$target/wasm32-wasip2/release/$name.wasm" "$dest"
echo "$dest $(stat -c %s "$dest") bytes"
