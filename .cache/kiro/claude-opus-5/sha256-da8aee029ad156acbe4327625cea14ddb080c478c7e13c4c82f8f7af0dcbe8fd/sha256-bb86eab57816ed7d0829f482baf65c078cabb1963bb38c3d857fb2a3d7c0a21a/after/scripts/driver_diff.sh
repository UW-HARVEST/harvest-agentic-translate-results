#!/bin/bash
# CONFIGS.md row 89: run the C `driver` and the Rust `driver` on the same
# (deterministic) input and compare their stdout byte-for-byte.
#
#   driver_diff.sh                     -> every configuration
#   driver_diff.sh blake simple 128f   -> one configuration
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"

run_one() {
  local b=$1 t=$2 s=$3
  local cdrv="$ROOT/c_build/$b-$t-$s/app/driver"

  if [ ! -x "$cdrv" ]; then
    "$ROOT/scripts/build_c.sh" "$b" "$t" "$s" >/dev/null || { echo "FAIL(cbuild) $b-$t-$s"; return 1; }
  fi

  ( cd "$CRATE" && timeout 600 cargo build --release --no-default-features \
      --features "$b,$t,$s" --bin driver > /tmp/spx_drv_build.log 2>&1 ) \
    || { echo "FAIL(build) $b-$t-$s"; tail -20 /tmp/spx_drv_build.log; return 1; }

  local co ro crc rrc
  co=$(LD_LIBRARY_PATH="$ROOT/c_build/$b-$t-$s/lib/$b:$ROOT/c_build/$b-$t-$s/app:/tmp/ossl_shim/lib" \
        timeout 600 "$cdrv" 2>/tmp/spx_drv_c.err); crc=$?
  ro=$(timeout 600 "$CRATE/target/release/driver" 2>/tmp/spx_drv_r.err); rrc=$?

  if [ "$crc" != "$rrc" ]; then
    echo "FAIL(exit) $b-$t-$s  C=$crc Rust=$rrc"
    head -3 /tmp/spx_drv_c.err /tmp/spx_drv_r.err
    return 1
  fi
  if [ "$co" != "$ro" ]; then
    echo "FAIL(stdout) $b-$t-$s"
    echo "  C   : $co"
    echo "  Rust: $ro"
    return 1
  fi
  echo "OK   $b-$t-$s  $co"
}

rc=0
if [ $# -eq 3 ]; then
  run_one "$1" "$2" "$3" || rc=1
else
  for b in haraka sha2 shake blake; do
    for t in robust simple; do
      for s in 128s 128f 192s 192f 256s 256f; do
        run_one "$b" "$t" "$s" || rc=1
      done
    done
  done
fi
exit $rc
