#!/usr/bin/env bash
# Build the Rust cdylib for a given feature set and stamp a per-configuration copy.
#
#   ./build_so.sh [--release] --no-default-features --features add,5
#
# `cargo test` does not build a cdylib, and `target/<profile>/libdriver.so` is a
# single path shared by every feature set, so a copy named after the configuration
# is made and its path echoed. The test harness reads it from $MD_RUST_SO.
set -eu
cd "$(dirname "$0")"

profile_dir="debug"
args=()
for a in "$@"; do
  [[ "$a" == "--release" ]] && profile_dir="release"
  args+=("$a")
done

cargo build "${args[@]}" >/dev/null

# Derive the tag the same way tests/common/mod.rs does, from the requested
# features: OP precedence mul > sub > add, REPEAT precedence 0,1,2,3,4,6,7 then 5.
feats=""
prev=""
for a in "${args[@]}"; do
  [[ "$prev" == "--features" ]] && feats="$feats,$a"
  prev="$a"
done
if [[ " ${args[*]} " == *" --all-features "* ]]; then
  feats="add,sub,mul,0,1,2,3,4,5,6,7"
fi
has() { [[ ",$feats," == *",$1,"* ]]; }

op=add
has sub && op=sub
has mul && op=mul

repeat=5
for r in 7 6 4 3 2 1 0; do has "$r" && repeat="$r"; done

src="target/$profile_dir/libdriver.so"
[[ -f "$src" ]] || { echo "cargo did not produce $src" >&2; exit 1; }
dst="target/$profile_dir/libdriver_${op}_${repeat}.so"
cp -f "$src" "$dst"
echo "$PWD/$dst"
