#!/usr/bin/env bash
# Full verification sweep: every Cargo feature combination x every build variant
# of both libraries. Run from the `translation/` directory.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"
cd "$CRATE" || exit 1

CARGO="cargo"
OFFLINE=""
if ! $CARGO metadata --format-version 1 >/dev/null 2>&1; then OFFLINE="--offline"; fi
$CARGO metadata --format-version 1 $OFFLINE >/dev/null 2>&1 || OFFLINE="--offline"

fail=0

# ---------------------------------------------------------------- feature combos
# Enumerate features declared in Cargo.toml (there are none for this crate, so the
# loop degenerates to the single default configuration -- but it is derived, not
# assumed).
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inblock=1; next}
  /^\[/ {inblock=0}
  inblock && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    sub(/[[:space:]]*=.*/, ""); if ($0 != "default") print
  }' Cargo.toml)

COMBOS=("default")
n=${#FEATURES[@]}
if (( n > 0 )); then
  COMBOS+=("--no-default-features")
  for ((mask=1; mask<(1<<n); mask++)); do
    set=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then set="${set:+$set,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features $set")
  done
fi

echo "== feature combinations discovered: ${#COMBOS[@]} =="
printf '   %s\n' "${COMBOS[@]}"

# ------------------------------------------------------------- C library variants
build_c () {  # $1 = tag, $2.. = extra cmake flags
  local tag="$1"; shift
  # Built outside c_src/ so nothing in the read-only C tree is touched.
  local bdir="$CRATE/target/c_variants/build_$tag"
  mkdir -p "$bdir" || return 1
  ( cd "$bdir" && cmake "$ROOT/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON "$@" >/dev/null \
      && cmake --build . >/dev/null ) || return 1
  find "$bdir" -maxdepth 1 -name 'lib*.so' | sort | head -1
}

declare -A C_SOS
C_SOS[default]="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | head -1)"
for v in "O0:-DCMAKE_C_FLAGS=-O0" "O2:-DCMAKE_C_FLAGS=-O2" "O3:-DCMAKE_C_FLAGS=-O3" "Os:-DCMAKE_C_FLAGS=-Os"; do
  tag="${v%%:*}"; flag="${v#*:}"
  so="$(build_c "$tag" "$flag")"
  if [[ -n "$so" ]]; then C_SOS[$tag]="$so"; else echo "WARN: C build $tag failed"; fi
done

# ------------------------------------------------------------------- the sweep
for combo in "${COMBOS[@]}"; do
  flags=""
  [[ "$combo" != "default" ]] && flags="$combo"
  for profile in debug release; do
    prof_flag=""
    [[ "$profile" == release ]] && prof_flag="--release"
    echo
    echo "### build: profile=$profile features='${flags:-<default>}'"
    # shellcheck disable=SC2086
    $CARGO build $OFFLINE $prof_flag $flags >/dev/null 2>&1 || { echo "BUILD FAILED"; fail=1; continue; }
    RSO="$CRATE/target/$profile/libgaussian_kernel_lib.so"
    [[ -f "$RSO" ]] || { echo "missing $RSO"; fail=1; continue; }
    for ctag in "${!C_SOS[@]}"; do
      CSO="${C_SOS[$ctag]}"
      printf '  vs C[%-7s] ... ' "$ctag"
      # shellcheck disable=SC2086
      out=$(DIFF_C_SO="$CSO" DIFF_RUST_SO="$RSO" \
            timeout 600 $CARGO test $OFFLINE $prof_flag $flags 2>&1)
      if grep -q 'FAILED\|error\[' <<<"$out"; then
        echo "FAIL"; grep -E 'panicked|divergence|^test .* FAILED|test result' <<<"$out" | head -20
        fail=1
      else
        echo "ok ($(grep -c '^test .* \.\.\. ok' <<<"$out") tests)"
      fi
    done
  done
done

echo
if (( fail )); then echo "RESULT: FAILURES PRESENT"; exit 1; else echo "RESULT: ALL CONFIGURATIONS PASS"; fi
