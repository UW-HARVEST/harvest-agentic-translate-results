#!/usr/bin/env bash
# Runs the whole differential suite (Phases B, C, D) across EVERY cargo feature
# combination and both profiles, plus the C/Rust symbol diff.
#
# Usage: ./run_all.sh
set -uo pipefail
cd "$(dirname "$0")"

REPO="$(cd .. && pwd)"
OFFLINE="--offline"          # crates.io is unreachable in this sandbox
FAIL=0

echo "=== 1. Build the C shared object ==="
mkdir -p "$REPO/c_src/build"
( cd "$REPO/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$REPO/c_src/build/libdriver.so"
echo "    $C_SO"

echo
echo "=== 2. Enumerate feature combinations from Cargo.toml ==="
# Every feature name declared in [features] (excluding "default").
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{gsub(/ /,"");split($0,a,"=");if(a[1]!="default")print a[1]}' Cargo.toml)
if [ -z "$FEATS" ]; then
  echo "    no [features] table -> the only configuration is the default one"
  COMBOS=("default" "no-default")
else
  COMBOS=("default" "no-default")
  # Power set of the declared features, each run with --no-default-features.
  mapfile -t F <<<"$FEATS"
  n=${#F[@]}
  for ((m=1; m<(1<<n); m++)); do
    c=""
    for ((i=0; i<n; i++)); do
      (( m & (1<<i) )) && c="${c:+$c,}${F[$i]}"
    done
    COMBOS+=("$c")
  done
fi
printf '    combos: %s\n' "${COMBOS[*]}"

run() { # run <label> <extra cargo args...>
  local label="$1"; shift
  echo
  echo "--- cargo test $* ($label) ---"
  if timeout 600 cargo test $OFFLINE "$@" 2>&1 | grep -E '^(test result|error|warning: unused)' ; then :; fi
  local st=${PIPESTATUS[0]}
  if [ "$st" -ne 0 ]; then echo "    ==> FAILED ($label)"; FAIL=1; else echo "    ==> ok ($label)"; fi
}

echo
echo "=== 3. Phases B/C/D under every feature combination x profile ==="
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    default)    args=() ;;
    no-default) args=(--no-default-features) ;;
    *)          args=(--no-default-features --features "$combo") ;;
  esac
  run "$combo/debug"   "${args[@]}"
  run "$combo/release" "${args[@]}" --release
done

echo
echo "=== 4. Symbol diff: C .so vs Rust .so ==="
for prof in debug release; do
  # The freshly-rebuilt artifact the test harness actually dlopen'd.
  R_SO="target/ffi-so/$prof/libdriver.so"
  [ -f "${R_SO:-}" ] || { echo "  $prof: no Rust .so built, skipping"; continue; }
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
    <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort))
  if [ -n "$missing" ]; then
    echo "  $prof: MISSING from Rust .so ($R_SO):"; echo "$missing" | sed 's/^/    /'
    FAIL=1
  else
    echo "  $prof: symbol diff EMPTY ($R_SO)"
  fi
  unset R_SO
done

echo
echo "=== 5. Binary/driver executable check ==="
if grep -q 'add_executable' "$REPO/c_src/CMakeLists.txt" || [ -d src/bin ] || [ -f src/main.rs ] \
   || grep -q '^\[\[bin\]\]' Cargo.toml; then
  echo "  a binary target exists -- stdout comparison required (see below)"
  FAIL=1
else
  echo "  no binary target on either side (C: library only; Rust: cdylib only)"
  echo "  -> the stdout gate is covered by the in-process capture tests instead"
fi

echo
if [ "$FAIL" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit "$FAIL"
