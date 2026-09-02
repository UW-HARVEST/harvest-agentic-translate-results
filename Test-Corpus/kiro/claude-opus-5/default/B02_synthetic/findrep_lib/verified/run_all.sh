#!/usr/bin/env bash
# Full verification sweep: Phase A symbol parity + Phases B/C differential tests
# under EVERY feature combination, plus the debug profile of the Rust .so.
#
# Usage: ./run_all.sh            (run from translation/)
set -uo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/.." && pwd)
cd "$HERE"

rc=0
say() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 0. Build the C shared library (ground truth).
# ---------------------------------------------------------------------------
say "building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
# 1. Enumerate every feature combination from Cargo.toml.
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ {inb=1; next}
  /^\[/ {inb=0}
  inb && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml | sort -u)

COMBOS=()
COMBOS+=("default:")                      # default features
COMBOS+=("no-default:")                   # --no-default-features, nothing on
if [ -n "$FEATURES" ]; then
  # every non-empty subset of the explicit features, on top of --no-default-features
  mapfile -t FARR <<< "$FEATURES"
  n=${#FARR[@]}
  for ((m=1; m<(1<<n); m++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( m & (1<<i) )); then sel="${sel:+$sel,}${FARR[i]}"; fi
    done
    COMBOS+=("no-default:$sel")
  done
  COMBOS+=("all:")                        # --all-features
fi

say "feature combinations to verify (${#COMBOS[@]})"
printf '  %s\n' "${COMBOS[@]}"
[ -n "$FEATURES" ] || echo "  (Cargo.toml declares no [features]; the set is the single empty combination)"

flags_for() {
  case "$1" in
    default:)      echo "" ;;
    all:)          echo "--all-features" ;;
    no-default:)   echo "--no-default-features" ;;
    no-default:*)  echo "--no-default-features --features ${1#no-default:}" ;;
  esac
}

# ---------------------------------------------------------------------------
# 2. cargo check every combination first (fast failure).
# ---------------------------------------------------------------------------
say "cargo check across all feature combinations"
for combo in "${COMBOS[@]}"; do
  f=$(flags_for "$combo")
  if timeout 300 cargo check --all-targets $f >/dev/null 2>&1; then
    echo "  ok      check $combo"
  else
    echo "  FAILED  check $combo"; rc=1
  fi
done

# ---------------------------------------------------------------------------
# 3. Per combination: build the .so, diff symbols, run every test.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  f=$(flags_for "$combo")
  for profile in release debug; do
    pflag=""; [ "$profile" = release ] && pflag="--release"

    say "combo=$combo profile=$profile"
    if ! timeout 600 cargo build $pflag $f >/dev/null 2>&1; then
      echo "  FAILED  build"; rc=1; continue
    fi
    R_SO="target/$profile/libfindrep_lib.so"
    if [ ! -f "$R_SO" ]; then echo "  FAILED  $R_SO missing"; rc=1; continue; fi

    # ---- Phase A / D: symbol parity -------------------------------------
    missing=$(comm -13 \
      <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u))
    if [ -z "$missing" ]; then
      cnt=$(nm -D --defined-only "$C_SO" | wc -l)
      echo "  ok      symbol parity: all $cnt C symbols exported by the Rust .so"
    else
      echo "  FAILED  symbols missing from the Rust .so:"; echo "$missing" | sed 's/^/            /'
      rc=1
    fi

    # ---- unresolved (non-libc) undefined symbols ------------------------
    unres=$(ldd -r "$R_SO" 2>&1 | grep -i 'undefined symbol' || true)
    if [ -z "$unres" ]; then
      echo "  ok      no unresolved undefined symbols"
    else
      echo "  FAILED  unresolved symbols:"; echo "$unres" | sed 's/^/            /'; rc=1
    fi

    # ---- Phases B + C: the differential suite ---------------------------
    # Always load the .so for THIS profile, whichever profile the harness runs in.
    if FINDREP_RUST_SO="$PWD/$R_SO" timeout 600 cargo test --release $f 2>&1 | tail -1 \
         | grep -q 'test result: ok'; then
      : # per-target summaries checked below
    fi
    out=$(FINDREP_RUST_SO="$PWD/$R_SO" timeout 600 cargo test --release $f 2>&1)
    if echo "$out" | grep -qE '^test result: FAILED|^error'; then
      echo "  FAILED  differential tests"
      echo "$out" | grep -E '^test result|FAILED|panicked' | sed 's/^/            /' | head -40
      rc=1
    else
      echo "$out" | grep -E '^test result: ok' | sed 's/^/  ok      /'
    fi
  done
done

# ---------------------------------------------------------------------------
# 4. Restore the release artifact and print the final symbol table.
# ---------------------------------------------------------------------------
say "final symbol diff (C .so vs Rust release .so)"
cargo build --release >/dev/null 2>&1
diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u) \
     <(nm -D --defined-only target/release/libfindrep_lib.so | awk '{print $NF}' | sort -u | grep -vE '^_ZN|^rust_|^__rust|^_R') \
  && echo "  identical exported-symbol sets"

say "RESULT"
if [ "$rc" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$rc"
