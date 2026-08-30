#!/bin/bash
# Build the C .so and the Rust .so for every feature combination, then diff
# their exported dynamic symbols.
set -u
W="$(cd "$(dirname "$0")/.." && pwd)"
RUSTLIB=lib005_sphincs_PQCgenKAT_sign_blake_128f_simple.so
mkdir -p "$W/work"
: > "$W/work/symdiff.txt"
fail=0
while IFS=, read -r b t s; do
  cfg="${b}_${s}_${t}"
  # --- C ---
  if ! "$W/scripts/build_c.sh" "$b" "$s" "$t" > "$W/work/cbuild_$cfg.log" 2>&1; then
    echo "C-BUILD-FAIL $cfg"; tail -5 "$W/work/cbuild_$cfg.log"; fail=1; continue
  fi
  # --- Rust ---
  (cd "$W/translation" && timeout 400 cargo build --release --quiet --no-default-features \
      --features "$b,$t,$s" > "$W/work/rbuild_$cfg.log" 2>&1)
  if [ $? -ne 0 ]; then
    echo "RUST-BUILD-FAIL $cfg"; tail -20 "$W/work/rbuild_$cfg.log"; fail=1; continue
  fi
  mkdir -p "$W/rustlibs"
  cp "$W/translation/target/release/$RUSTLIB" "$W/rustlibs/librust_$cfg.so"

  nm -D --defined-only "$W/cbuild/$cfg/libc_sphincs.so" | awk '{print $3}' | sort -u > "$W/work/c_$cfg.syms"
  nm -D --defined-only "$W/rustlibs/librust_$cfg.so"    | awk '{print $3}' | sort -u > "$W/work/r_$cfg.syms"
  miss=$(comm -23 "$W/work/c_$cfg.syms" "$W/work/r_$cfg.syms" | tr '\n' ' ')
  extra=$(comm -13 "$W/work/c_$cfg.syms" "$W/work/r_$cfg.syms" | tr '\n' ' ')
  if [ -n "$miss" ] || [ -n "$extra" ]; then
    echo "SYMDIFF $cfg missing=[$miss] extra=[$extra]" | tee -a "$W/work/symdiff.txt"
    fail=1
  else
    echo "ok      $cfg ($(wc -l < "$W/work/c_$cfg.syms") symbols)"
  fi
done < <("$W/scripts/all_combos.sh")
exit $fail
