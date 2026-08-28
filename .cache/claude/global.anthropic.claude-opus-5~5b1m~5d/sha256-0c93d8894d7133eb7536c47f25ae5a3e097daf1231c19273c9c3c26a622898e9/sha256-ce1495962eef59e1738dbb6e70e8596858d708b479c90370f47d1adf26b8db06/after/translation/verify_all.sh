#!/usr/bin/env bash
# Completion gate: run the full differential suite for EVERY cargo feature
# combination and for both profiles, plus the raw `nm -D` symbol diff.
#
# Usage:  ./verify_all.sh      (run from the crate root, i.e. translation/)
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CARGO="cargo --offline"
SCRATCH="$(mktemp -d "${TMPDIR:-target}/hdrverify.XXXXXX")" || exit 1
trap 'rm -rf "$SCRATCH"' EXIT
fail=0
summary=()

note() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
record() { summary+=("$1"); [ "${1:0:4}" = "FAIL" ] && fail=1; return 0; }

# ---------------------------------------------------------------------------
# 0. Build the C shared library (name is derived from the parent dir name).
# ---------------------------------------------------------------------------
note "Building the C shared library"
C_SRC="$(cd .. && pwd)/c_src"
mkdir -p "$C_SRC/build"
( cd "$C_SRC/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$C_SRC"/build/lib*.so | head -1)"
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
note "Enumerating cargo feature combinations"
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/ {inside=0}
  inside && /^[A-Za-z0-9_-]+[ \t]*=/ {
    split($0, a, "="); gsub(/[ \t]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=()
COMBOS+=("default|")                       # default features
COMBOS+=("no-default|--no-default-features")
if [ -n "$FEATURES" ]; then
  for f in $FEATURES; do
    COMBOS+=("$f|--no-default-features --features $f")
  done
  # all features together
  ALL=$(echo "$FEATURES" | paste -sd, -)
  COMBOS+=("all|--no-default-features --features $ALL")
  COMBOS+=("all-plus-default|--all-features")
fi
echo "declared features: ${FEATURES:-<none>}"
printf 'combination: %s\n' "${COMBOS[@]%%|*}"

# ---------------------------------------------------------------------------
# 2. cargo check + full test suite for each combination x profile.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  name="${combo%%|*}"
  flags="${combo#*|}"
  for profile in dev release; do
    relflag=""
    [ "$profile" = release ] && relflag="--release"

    note "check: features=$name profile=$profile"
    if $CARGO check --tests $flags $relflag >/dev/null 2>&1; then
      record "OK   check  features=$name profile=$profile"
    else
      $CARGO check --tests $flags $relflag 2>&1 | tail -20
      record "FAIL check  features=$name profile=$profile"
      continue
    fi

    # The lib is a cdylib only, which `cargo test` does not build by itself.
    $CARGO build $flags $relflag >/dev/null 2>&1 \
      || { record "FAIL build  features=$name profile=$profile"; continue; }

    note "test: features=$name profile=$profile"
    timeout 600 $CARGO test $flags $relflag > "$SCRATCH/test_out" 2>&1
    trc=$?
    tail -25 "$SCRATCH/test_out"
    okn=$(grep -c 'test result: ok' "$SCRATCH/test_out")
    badn=$(grep -c 'test result: FAILED' "$SCRATCH/test_out")
    tests=$(awk '/test result: ok/ {n+=$4} END {print n+0}' "$SCRATCH/test_out")
    if [ "$trc" -eq 0 ] && [ "$badn" -eq 0 ] && [ "$okn" -ge 3 ]; then
      record "OK   test   features=$name profile=$profile ($okn suites, $tests tests ok)"
    else
      record "FAIL test   features=$name profile=$profile (rc=$trc ok_suites=$okn failed_suites=$badn)"
    fi

    # ------------------------------------------------------------------
    # 3. Raw nm -D symbol diff for this combination/profile.
    # ------------------------------------------------------------------
    tdir="target/debug"; [ "$profile" = release ] && tdir="target/release"
    R_SO="$tdir/libhdr_bitrate_lib.so"
    if [ ! -f "$R_SO" ]; then
      record "FAIL nm     features=$name profile=$profile (no $R_SO)"
      continue
    fi
    nm -D --defined-only "$C_SO" | awk 'NF>=2 {print $NF}' | sort -u > "$SCRATCH/c_syms"
    nm -D --defined-only "$R_SO" | awk 'NF>=2 {print $NF}' | sort -u > "$SCRATCH/r_syms"
    missing=$(comm -23 "$SCRATCH/c_syms" "$SCRATCH/r_syms")
    nC=$(wc -l < "$SCRATCH/c_syms" | tr -d " "); nR=$(wc -l < "$SCRATCH/r_syms" | tr -d " ")
    if [ -z "$missing" ]; then
      record "OK   nm     features=$name profile=$profile (C=$nC exported, Rust=$nR, missing=0)"
    else
      echo "MISSING FROM RUST .so:"; echo "$missing"
      record "FAIL nm     features=$name profile=$profile (missing: $(echo "$missing" | tr '\n' ' '))"
    fi
  done
done

note "SUMMARY"
printf '%s\n' "${summary[@]}"
if [ "$fail" -eq 0 ]; then
  printf '\n\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\n\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$fail"
