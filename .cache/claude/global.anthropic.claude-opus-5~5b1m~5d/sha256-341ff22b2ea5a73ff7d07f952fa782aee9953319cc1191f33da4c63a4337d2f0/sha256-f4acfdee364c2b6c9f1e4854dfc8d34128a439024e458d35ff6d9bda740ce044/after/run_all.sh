#!/bin/bash
# End-to-end differential verification across EVERY feature combination.
#
#   ./run_all.sh                 # all 60 Rust feature combinations
#   BACKENDS=blake ./run_all.sh   # just one backend
#
# For each combination this
#   1. builds the C shared libraries (CMake, plus one combined .so),
#   2. builds the Rust cdylib with the matching features,
#   3. checks nm -D symbol parity,
#   4. runs the Phase B (configs.rs) and Phase C (errors.rs) differential tests.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"

# `shake256` is a Cargo alias for `shake` and maps onto the same C configuration.
RUST_BACKENDS="${RUST_BACKENDS:-haraka sha2 shake shake256 blake}"
THASHES="${THASHES:-robust simple}"
SECPARS="${SECPARS:-128s 128f 192s 192f 256s 256f}"

c_backend() { case "$1" in shake256) echo shake ;; *) echo "$1" ;; esac; }

# ---------------------------------------------------------------------------
# 1. C libraries (all 48 CMake configurations) - cached between runs
# ---------------------------------------------------------------------------
echo "=== building the C reference libraries ==="
BACKENDS="$(for b in $RUST_BACKENDS; do c_backend "$b"; done | sort -u | tr '\n' ' ')" \
  THASHES="$THASHES" SECPARS="$SECPARS" "$ROOT/build_c_all.sh" | grep -c '^ok\|^skip' \
  | xargs -I{} echo "  {} C configurations ready"

pass=0; fail=0; failed_combos=""
for b in $RUST_BACKENDS; do
  for t in $THASHES; do
    for s in $SECPARS; do
      feats="$b,$t,$s"
      cb="$(c_backend "$b")"
      combo="$cb-$t-$s"
      printf '=== %-24s (C config %s)\n' "$feats" "$combo"

      # 2. Rust cdylib for exactly these features
      mkdir -p "$ROOT/rbuild/$combo"
      if ! ( cd "$ROOT/translation" && cargo build --release --offline --quiet \
               --no-default-features --features "$feats" ); then
        echo "  BUILD FAILED"; fail=$((fail+1)); failed_combos="$failed_combos $feats(build)"; continue
      fi
      cp "$ROOT/translation/target/release/libsphincs_core_det.so" "$ROOT/rbuild/$combo/"

      # 3. symbol parity for this configuration
      T="${TMPDIR:-/tmp}"
      nm -D --defined-only "$ROOT/cbuild/$combo/libsphincs_core_det.so" \
                           "$ROOT/cbuild/$combo/lib$cb.so" \
        | awk 'NF>=3{print $3}' | sort -u > "$T/ra_c.txt"
      nm -D --defined-only "$ROOT/rbuild/$combo/libsphincs_core_det.so" \
        | awk 'NF>=3{print $3}' | sort -u > "$T/ra_r.txt"
      miss=$(comm -23 "$T/ra_c.txt" "$T/ra_r.txt" | tr '\n' ' ')
      if [ -n "${miss// /}" ]; then
        echo "  SYMBOL PARITY FAILED, missing: $miss"
        fail=$((fail+1)); failed_combos="$failed_combos $feats(symbols)"; continue
      fi
      echo "  symbols: $(wc -l < "$T/ra_c.txt") C symbols, all present in the Rust .so"

      # 4. Phase B + Phase C differential tests
      ok=1
      for suite in configs errors; do
        if timeout 600 env -C "$ROOT/translation" cargo test --offline --quiet \
             --no-default-features --features "$feats" --test "$suite" \
             > "$T/out_$suite.log" 2>&1; then
          echo "  $suite: $(grep -oE '[0-9]+ passed' "$T/out_$suite.log" | head -1)"
        else
          echo "  $suite: FAILED"
          tail -25 "$T/out_$suite.log"
          ok=0
        fi
      done
      if [ $ok = 1 ]; then pass=$((pass+1)); else
        fail=$((fail+1)); failed_combos="$failed_combos $feats(tests)"
      fi
    done
  done
done

echo
echo "==================== SUMMARY ===================="
echo "feature combinations fully passing: $pass"
echo "feature combinations failing:       $fail"
[ -n "$failed_combos" ] && echo "failures:$failed_combos"
exit $((fail > 0))
