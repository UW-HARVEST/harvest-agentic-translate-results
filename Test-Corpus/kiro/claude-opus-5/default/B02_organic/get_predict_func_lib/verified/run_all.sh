#!/usr/bin/env bash
# Full verification sweep: every feature combination x every cdylib build
# configuration. Run from the `translation/` directory.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

banner() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 0. Build the C reference library.
# ---------------------------------------------------------------------------
banner "Building C reference library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
banner "Enumerating feature combinations"
FEATURES="$(cargo metadata --no-deps --format-version 1 2>/dev/null \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); print(" ".join(k for k in d["packages"][0]["features"] if k!="default"))')"
if [ -z "${FEATURES// /}" ]; then
  echo "no optional features declared -> the only configurations are"
  echo "  (a) default            (b) --no-default-features"
  COMBOS=("" "--no-default-features")
else
  echo "features: $FEATURES"
  # Full power set.
  COMBOS=("" "--no-default-features")
  # shellcheck disable=SC2206
  read -r -a FARR <<< "$FEATURES"
  n=${#FARR[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=""
    for ((b=0; b<n; b++)); do
      if (( mask & (1<<b) )); then sel="$sel,${FARR[$b]}"; fi
    done
    COMBOS+=("--no-default-features --features ${sel#,}")
    COMBOS+=("--features ${sel#,}")
  done
fi

# ---------------------------------------------------------------------------
# 2. cargo check every combination.
# ---------------------------------------------------------------------------
banner "cargo check across feature combinations"
for combo in "${COMBOS[@]}"; do
  desc="${combo:-<default>}"
  if timeout 600 cargo check --quiet $combo 2>&1 | tail -5; then
    echo "  check OK   : $desc"
  else
    echo "  check FAIL : $desc"; FAIL=1
  fi
done

# ---------------------------------------------------------------------------
# 3. cargo test every (feature combo x cdylib build configuration).
#
# Pointer identity is what get_predict_func is built on, and optimisation
# level / LTO / codegen-units / identical-code-folding are precisely what can
# perturb it -- so the .so is rebuilt and retested under each.
# ---------------------------------------------------------------------------
declare -a SO_CFG_TAG=(
  "release-default"
  "dev-unopt"
  "release-lto-fat-cgu1"
  "release-opt-z"
)
declare -a SO_CFG_PROFILE=(
  "release"
  "dev"
  "release"
  "release"
)
declare -a SO_CFG_ARGS=(
  ""
  ""
  "--config profile.release.lto=\"fat\" --config profile.release.codegen-units=1"
  "--config profile.release.opt-level=\"z\""
)

banner "cargo test across feature combinations x cdylib build configurations"
for combo in "${COMBOS[@]}"; do
  for i in "${!SO_CFG_TAG[@]}"; do
    tag="${SO_CFG_TAG[$i]}"
    desc="${combo:-<default>} | so=${tag}"
    out=$(FFI_SO_TAG="$tag" \
          FFI_SO_PROFILE="${SO_CFG_PROFILE[$i]}" \
          FFI_SO_EXTRA_CARGO_ARGS="${SO_CFG_ARGS[$i]}" \
          timeout 600 cargo test --quiet $combo 2>&1)
    if [ $? -eq 0 ]; then
      echo "  test OK    : $desc"
    else
      echo "  test FAIL  : $desc"
      echo "$out" | tail -30
      FAIL=1
    fi
  done
done

# ---------------------------------------------------------------------------
# 4. Symbol parity, checked directly with nm for every cdylib configuration.
# ---------------------------------------------------------------------------
banner "nm -D symbol diff (C vs Rust) per cdylib configuration"
for i in "${!SO_CFG_TAG[@]}"; do
  tag="${SO_CFG_TAG[$i]}"
  prof="${SO_CFG_PROFILE[$i]}"
  outdir=$([ "$prof" = "dev" ] && echo debug || echo "$prof")
  so="target/ffi-so-${tag}/${outdir}/libget_predict_func_lib.so"
  if [ ! -f "$so" ]; then echo "  MISSING    : $so"; FAIL=1; continue; fi
  diff_out=$(comm -3 \
    <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only "$so"   | awk '{print $NF}' | sort -u))
  if [ -z "$diff_out" ]; then
    echo "  parity OK  : $tag"
  else
    echo "  parity FAIL: $tag"; echo "$diff_out"; FAIL=1
  fi
  undef=$(nm -D -u "$so" | grep -v -E '@GLIBC|@GCC|_ITM_|__gmon_start__')
  if [ -n "$undef" ]; then
    echo "  undefined non-libc symbols in $tag:"; echo "$undef"; FAIL=1
  fi
done

# ---------------------------------------------------------------------------
# 5. Optional: exhaustive 2^32 sweep per cdylib configuration.
#    Enable with `./run_all.sh --exhaustive` (~35 s per configuration).
# ---------------------------------------------------------------------------
if [ "${1:-}" = "--exhaustive" ]; then
  banner "exhaustive 2^32 differential sweep per cdylib configuration"
  for i in "${!SO_CFG_TAG[@]}"; do
    tag="${SO_CFG_TAG[$i]}"
    printf '  %-24s ' "$tag"
    out=$(FFI_SO_TAG="$tag" \
          FFI_SO_PROFILE="${SO_CFG_PROFILE[$i]}" \
          FFI_SO_EXTRA_CARGO_ARGS="${SO_CFG_ARGS[$i]}" \
          timeout 600 cargo test --release --quiet --test exhaustive \
            -- --ignored --nocapture 2>&1)
    if [ $? -eq 0 ]; then
      echo "OK  ($(echo "$out" | grep -o '[0-9]* values checked'))"
    else
      echo "FAIL"; echo "$out" | tail -20; FAIL=1
    fi
  done
fi

banner "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"