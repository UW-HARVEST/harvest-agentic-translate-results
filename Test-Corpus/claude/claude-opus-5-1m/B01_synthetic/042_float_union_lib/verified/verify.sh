#!/usr/bin/env bash
# Full verification driver: enumerates every Cargo feature combination,
# type-checks, builds and differential-tests each one, and diffs the exported
# symbols of the C and Rust shared objects.
#
# Usage: ./verify.sh
set -uo pipefail

cd "$(dirname "$0")"
: "${TMPDIR:=/tmp}"
LOG="$TMPDIR/verify.$$.log"
rc=0

hr() { printf '%s\n' "------------------------------------------------------------"; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations (powerset of the declared features).
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1])
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

n=${#FEATURES[@]}
echo "Declared features (${n}): ${FEATURES[*]:-<none>}"

COMBOS=()
if (( n == 0 )); then
  COMBOS=("")                       # the single (default == no-default) config
else
  for (( mask = 0; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "Feature combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. Build the C reference shared object.
# ---------------------------------------------------------------------------
hr
echo "Building the C reference library"
mkdir -p c_src/build
( cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    && cmake --build . ) >"$LOG" 2>&1 || { echo "C BUILD FAILED"; tail -20 "$LOG"; exit 1; }
C_SO=c_src/build/libdriver.so
echo "  -> $C_SO"

# ---------------------------------------------------------------------------
# 3. Per-combination: check, build, test.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  hr
  label="${combo:-<no features>}"
  echo "### combination: $label"

  args=(--no-default-features)
  [[ -n "$combo" ]] && args+=(--features "$combo")

  for stage in check build test; do
    if timeout 600 cargo "$stage" "${args[@]}" >"$LOG" 2>&1; then
      if [[ $stage == test ]]; then
        grep -hE '^test result:' "$LOG" | sed 's/^/    /'
      fi
      echo "  cargo $stage: OK"
    else
      echo "  cargo $stage: FAILED"
      tail -40 "$LOG"
      rc=1
    fi
  done

  # ---- symbol parity for this combination -------------------------------
  RUST_SO=target/debug/libdriver.so
  if [[ ! -f $RUST_SO ]]; then
    echo "  Rust .so missing at $RUST_SO"; rc=1; continue
  fi
  nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort -u >"$TMPDIR/c.syms"
  nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u >"$TMPDIR/r.syms"
  missing=$(comm -23 "$TMPDIR/c.syms" "$TMPDIR/r.syms")
  echo "  C exports:    $(wc -l <"$TMPDIR/c.syms")  [$(tr '\n' ' ' <"$TMPDIR/c.syms")]"
  echo "  Rust exports: $(wc -l <"$TMPDIR/r.syms")  [$(tr '\n' ' ' <"$TMPDIR/r.syms")]"
  if [[ -n $missing ]]; then
    echo "  SYMBOL PARITY FAILED — missing from the Rust .so:"
    printf '    %s\n' $missing
    rc=1
  else
    echo "  symbol parity: OK (0 missing)"
  fi

  # ---- undefined non-libc symbols in the Rust .so -----------------------
  undef=$(nm -D --undefined-only "$RUST_SO" \
            | awk '{print $NF}' \
            | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^_Unwind_|^statx$|^gettid$' || true)
  if [[ -n $undef ]]; then
    echo "  UNRESOLVED non-libc symbols in the Rust .so:"
    printf '    %s\n' $undef
    rc=1
  else
    echo "  undefined non-libc symbols: 0"
  fi
done

hr
if (( rc == 0 )); then
  echo "ALL VERIFICATION PASSED"
else
  echo "VERIFICATION FAILED"
fi
rm -f "$LOG"
exit $rc
