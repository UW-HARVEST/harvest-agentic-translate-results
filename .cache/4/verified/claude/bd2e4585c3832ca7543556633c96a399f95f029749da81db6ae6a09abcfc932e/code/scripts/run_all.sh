#!/bin/bash
# Full verification driver.
#   stage 0: cargo check every feature combination
#   stage 1: build the C .so/.exe and the Rust .so/.exe for all 24 configs
#   stage 2: nm -D symbol parity for all 24 configs
#   stage 3: Phase B + Phase C differential tests for all 24 configs
#   stage 4: "cache variable unset" default-fallback + repeat_N alias configs
#   stage 5: C-executable vs Rust-executable output diff for all 24 configs
#   stage 6: same, but for the optimised `--release` profile (panic = "abort")
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT" || exit 1
fail=0
# Optional args select which stages run (default: all). Stage 3/4 accept an
# OP filter, e.g.  run_all.sh 3:add   or   run_all.sh 0 1 2
WANT="${*:-0 1 2 3 4 5 6}"
want() { case " $WANT " in *" $1 "*) return 0;; *" $1:"*) return 0;; esac; return 1; }
opfilter() { for w in $WANT; do case "$w" in "$1":*) echo "${w#*:}"; return;; esac; done; echo ""; }
say() { printf '\n===== %s =====\n' "$*"; }

if want 0; then
say "stage 0: cargo check all feature combinations"
bash scripts/check_all.sh | tail -3 || fail=1
fi

if want 1; then
say "stage 1: build artifacts for all 24 configurations"
bash scripts/build_artifacts.sh > "$TMPDIR/build_all.log" 2>&1 \
  && echo "built $(ls artifacts | wc -l) configurations" || { cat "$TMPDIR/build_all.log"; fail=1; }
fi

if want 2; then
say "stage 2: nm -D symbol parity"
bash scripts/symdiff.sh | tail -2 || fail=1
fi

if want 3; then
F3="$(opfilter 3)"
say "stage 3: differential tests (Phase B + Phase C)${F3:+ [OP=$F3 only]}"
for c in $(./scripts/combos.sh); do
  op="${c%,*}"; r="${c#*,}"
  [ -n "$F3" ] && [ "$op" != "$F3" ] && continue
  out="$(timeout 600 cargo test --offline --quiet --no-default-features --features "$op,$r" \
          --test differential -- --test-threads=1 2>&1)"
  if printf '%s' "$out" | grep -q 'test result: ok'; then
    n="$(printf '%s' "$out" | grep -oP '\d+(?= passed)' | head -1)"
    echo "OK   OP=$op REPEAT=$r  ($n tests)"
  else
    echo "FAIL OP=$op REPEAT=$r"; printf '%s\n' "$out" | tail -30; fail=1
  fi
done
fi

if want 4; then
F4="$(opfilter 4)"
say "stage 4: default-fallback and alias feature spellings${F4:+ [filter=$F4]}"
# The Rust artifacts are rebuilt with the alternative spelling and compared
# against the C build for the (OP, REPEAT) that spelling must resolve to:
#   `-DOP` unset  => add        `-DREPEAT` unset => 5
#   repeat_N alias must select the same REPEAT as the bare feature N
declare -a SPECIAL=()
for op in add sub mul; do SPECIAL+=("$op|${op}_5"); done
for r in 0 1 2 3 4 5 6 7; do SPECIAL+=("$r|add_$r"); done
for op in add sub mul; do for r in 0 1 2 3 4 5 6 7; do
  SPECIAL+=("$op,repeat_$r|${op}_$r"); done; done
SPECIAL+=("default|add_5")
scratch="$ROOT/artifacts/.special-$$"   # unique per run: never share between runs
for s in "${SPECIAL[@]}"; do
  feats="${s%|*}"; expect="${s#*|}"
  if [ "$F4" = rest ]; then
    # only the "cache variable unset" spellings: bare REPEAT numbers + `default`
    case "$feats" in *add*|*sub*|*mul*) continue;; esac
  elif [ -n "$F4" ]; then
    [[ "$feats" != *"$F4"* ]] && continue
  fi
  rm -rf "$scratch"; mkdir -p "$scratch/cbin" "$scratch/rbin"
  # the C side of the comparison: the build for the (OP, REPEAT) it must equal
  cp "$ROOT/artifacts/$expect/libcdriver.so" "$scratch/libcdriver.so"
  cp "$ROOT/artifacts/$expect/cbin/driver"   "$scratch/cbin/driver"
  # the Rust side: freshly built with the alternative feature spelling
  if ! cargo build --offline --quiet --no-default-features --features "$feats" 2>"$TMPDIR/sp.log"; then
    echo "FAIL cargo build --features $feats"; cat "$TMPDIR/sp.log"; fail=1; continue
  fi
  cp target/debug/libdriver.so "$scratch/libdriver.so"
  cp target/debug/driver       "$scratch/rbin/driver"
  # symbol parity for this spelling too
  cn="$(nm -D --defined-only "$scratch/libcdriver.so" | awk '{print $2, $3}' | sort)"
  rn="$(nm -D --defined-only "$scratch/libdriver.so"  | awk '{print $2, $3}' | sort)"
  if [ "$cn" != "$rn" ]; then
    echo "FAIL symbols for --features $feats"; diff <(printf '%s\n' "$cn") <(printf '%s\n' "$rn"); fail=1
  fi
  out="$(MD_ARTIFACT_DIR="$scratch" timeout 600 cargo test --offline --quiet \
          --no-default-features --features "$feats" --test differential -- \
          --test-threads=1 2>&1)"
  if printf '%s' "$out" | grep -q 'test result: ok'; then
    n="$(printf '%s' "$out" | grep -oP '\d+(?= passed)' | head -1)"
    echo "OK   --features $feats  vs C build $expect  ($n tests)"
  else
    echo "FAIL --features $feats  vs C build $expect"; printf '%s\n' "$out" | tail -25; fail=1
  fi
done
rm -rf "$scratch"
fi

if want 5; then
say "stage 5: C executable vs Rust executable, all configurations"
bash scripts/diff_bins.sh | tail -2 || fail=1
fi

if want 6; then
say "stage 6: release profile (optimised, panic = abort) build + symbols + output"
bash scripts/release_check.sh | tail -2 || fail=1
fi

say "RESULT"
[ $fail -eq 0 ] && echo "ALL VERIFICATION STAGES PASSED" || echo "VERIFICATION FAILURES PRESENT"
exit $fail
