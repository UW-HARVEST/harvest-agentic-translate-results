#!/usr/bin/env bash
# Negative control for the differential suite.
#
# An all-green test run only means something if the suite can actually *fail*.
# This script builds several deliberately mutated copies of the C library (in a
# scratch directory — c_src is never touched) and feeds each one to the suite in
# place of the Rust `.so`. Every mutant MUST be caught.
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$CRATE_DIR/.." && pwd)"
C_SRC="$ROOT/c_src"
WORK="$CRATE_DIR/target/mutants"
cd "$CRATE_DIR"

C_SO="$C_SRC/build/libdriver.so"
if [ ! -f "$C_SO" ]; then
  echo "build the C library first (scripts/run_all.sh)"; exit 1
fi

rm -rf "$WORK"; mkdir -p "$WORK"

# ---------------------------------------------------------------- mutations
# name|file|sed expression
MUTANTS=(
  'm1_row_message|matrix.c|s/Insufficient rows in input string/Not enough rows in input string/'
  'm2_multiply_transpose|matrix.c|s/mat_a->matrix\[i\]\[k\] \* mat_b->matrix\[k\]\[j\]/mat_a->matrix[i][k] * mat_b->matrix[j][k]/'
  'm3_einval_to_eperm|write.c|s/return EINVAL;/return EPERM;/'
  'm4_buffer_size_off_by_one|matrix.c|s/+ mat->height + 1;/+ mat->height + 2;/'
  'm5_extra_separator|matrix.c|s/if (j < mat->width - 1)/if (j <= mat->width - 1)/'
  'm6_column_message_index|matrix.c|s/, i + 1);/, i);/'
  'm7_negative_height_abs|matrix.c|s/malloc(height \* sizeof(int\*))/malloc((height < 0 ? -height : height) * sizeof(int*))/'
  'm8_atoi_offset|matrix.c|s/= atoi(col_token);/= atoi(col_token) + 1;/'
)

pass=0; miss=0
for spec in "${MUTANTS[@]}"; do
  IFS='|' read -r name file expr <<<"$spec"
  d="$WORK/$name"
  mkdir -p "$d/src" "$d/include"
  cp "$C_SRC"/src/*.c "$d/src/"
  cp "$C_SRC"/include/*.h "$d/include/"
  before=$(md5sum "$d/src/$file" | cut -d' ' -f1)
  sed -i "$expr" "$d/src/$file"
  after=$(md5sum "$d/src/$file" | cut -d' ' -f1)
  if [ "$before" = "$after" ]; then
    echo "!! $name : mutation did not apply (bad sed expression) -- INVALID CONTROL"
    miss=$((miss+1)); continue
  fi

  if ! gcc -shared -fPIC -o "$d/libdriver.so" "$d"/src/*.c -I"$d/include" 2>"$d/build.log"; then
    echo "!! $name : mutant failed to compile"; sed -n 1,10p "$d/build.log"
    miss=$((miss+1)); continue
  fi

  out="$d/run.log"
  DRIVER_C_SO="$C_SO" DRIVER_RUST_SO="$d/libdriver.so" DIFFTEST_UB_STRICT=1 \
    timeout 900 cargo test -- --test-threads=1 >"$out" 2>&1
  rc=$?
  caught=$(grep -c '^test .* FAILED' "$out")
  if [ "$rc" -ne 0 ]; then
    echo "OK   $name : CAUGHT (exit=$rc, $caught failing tests)"
    grep '^test .* FAILED' "$out" | sed 's/^/       /' | head -6
    pass=$((pass+1))
  else
    echo "!! $name : NOT CAUGHT -- the suite is blind to this divergence"
    miss=$((miss+1))
  fi
done

echo
echo "negative control: $pass caught, $miss missed"
[ "$miss" -eq 0 ] || exit 1
