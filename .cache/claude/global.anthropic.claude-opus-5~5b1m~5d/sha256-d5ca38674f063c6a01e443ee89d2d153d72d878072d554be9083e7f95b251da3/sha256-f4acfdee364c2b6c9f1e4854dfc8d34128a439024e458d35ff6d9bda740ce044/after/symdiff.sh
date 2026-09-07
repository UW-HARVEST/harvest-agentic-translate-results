#!/bin/bash
# Compare exported symbols of the C .so set against the Rust cdylib.
# Usage: ./symdiff.sh <backend> <secpar> <thash>
R="$(cd "$(dirname "$0")" && pwd)"
B=${1:-blake}; S=${2:-128f}; T=${3:-simple}
CB="$R/cbuild/$B-$S-$T"
[ -d "$CB" ] || "$R/build_c.sh" "$B" "$S" "$T" >/dev/null
cd "$R/translation"
cargo build --release --no-default-features --features "$B,$T,$S" >/dev/null 2>&1 || { echo "rust build failed"; exit 1; }
csyms=$( { nm -D --defined-only "$CB/app/libsphincs_core_det.so"; nm -D --defined-only "$CB/lib/$B/lib$B.so"; } \
  | awk '{print $3}' | grep -v '^$' | sort -u )
rsyms=$(nm -D --defined-only "$R/translation/target/release/libsphincs_core_det.so" | awk '{print $3}' | sort -u)
echo "### $B-$S-$T"
echo "-- in C but NOT in Rust:"
comm -23 <(echo "$csyms") <(echo "$rsyms")
echo "-- (count C=$(echo "$csyms"|wc -l) Rust=$(echo "$rsyms"|wc -l))"
