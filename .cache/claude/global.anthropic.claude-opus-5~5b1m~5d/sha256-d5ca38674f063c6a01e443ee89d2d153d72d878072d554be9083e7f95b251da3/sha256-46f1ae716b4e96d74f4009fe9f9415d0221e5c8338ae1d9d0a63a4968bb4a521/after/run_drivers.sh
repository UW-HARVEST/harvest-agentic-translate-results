#!/bin/bash
# CONFIGS.md row 73: compare the C driver's and the Rust driver's stdout
# byte-for-byte, for every configuration, and cross-check both against
# reference_outputs.txt.
#
# The Rust driver is COPIED to a per-configuration path before being run, so a
# concurrent `cargo build` for another feature set cannot swap the binary out
# from under the comparison.
ROOT="$(cd "$(dirname "$0")" && pwd)"
export CARGO_NET_OFFLINE=1
export CARGO_TARGET_DIR="$ROOT/translation/target-drv"
BIN_DIR="$ROOT/gen/bin"; mkdir -p "$BIN_DIR" "$ROOT/gen"

BACKENDS="${BACKENDS:-blake haraka sha2 shake}"
THASHES="${THASHES:-robust simple}"
SECPARS="${SECPARS:-128s 128f 192s 192f 256s 256f}"
fail=0
for b in $BACKENDS; do for t in $THASHES; do for s in $SECPARS; do
  tag="$b-$t-$s"
  cd "$ROOT/translation" || exit 1
  if ! cargo build --release --no-default-features --features "$b,$t,$s" \
        > "$ROOT/gen/drvbuild-$tag.log" 2>&1; then
    echo "RUSTBUILD-FAIL $tag"; fail=1; continue
  fi
  cp "$CARGO_TARGET_DIR/release/driver" "$BIN_DIR/driver-$tag" || {
    echo "COPY-FAIL $tag"; fail=1; continue; }
  cout=$("$ROOT/cbuild/$tag/app/driver" 2>/dev/null); crc=$?
  rout=$("$BIN_DIR/driver-$tag" 2>/dev/null); rrc=$?
  ref=$(grep -P "^$b $t $s\t" "$ROOT/reference_outputs.txt" | cut -f2)
  if [ "$cout" != "$rout" ]; then
    echo "STDOUT-DIFF $tag"; echo "  C   =[$cout]"; echo "  Rust=[$rout]"; fail=1
  elif [ "$crc" != "$rrc" ]; then
    echo "EXIT-DIFF $tag  C=$crc Rust=$rrc"; fail=1
  elif [ -n "$ref" ] && [ "$cout" != "$ref" ]; then
    echo "REF-DIFF $tag"; echo "  got=[$cout]"; echo "  ref=[$ref]"; fail=1
  else
    echo "OK $tag exit=$crc $cout"
  fi
done; done; done
if [ "$fail" = 0 ]; then echo "ALL DRIVERS MATCH"; else echo "SOME DRIVERS DIFFER"; fi
exit $fail
