#!/usr/bin/env bash
# Full verification driver: C-vs-Rust differential testing across every
# build-time configuration (Phases A-D).
#
#   ./verify.sh            # everything
#   ./verify.sh --quick    # skip the exhaustive/oversized rows
#
# Never modifies anything under c_src/.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

CARGO_FLAGS="--offline"
FAILED=0
step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '   \033[31mFAIL\033[0m %s\n' "$*"; FAILED=$((FAILED + 1)); }

# ---------------------------------------------------------------------------
# Phase A.1 — enumerate every valid feature combination from Cargo.toml.
# ---------------------------------------------------------------------------
step "Enumerating feature combinations declared in Cargo.toml"
mapfile -t FEATURES < <(python3 - <<'PY'
import re, itertools, sys
src = open("Cargo.toml").read()
m = re.search(r"^\[features\]\s*$(.*?)(?=^\[|\Z)", src, re.S | re.M)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=')[0].strip()
        if name != "default":
            feats.append(name)
print("\n".join(feats))
PY
)
# All non-default features (none here) -> full power set of combinations.
COMBOS=("--no-default-features" "--no-default-features --features default" "")
if [ "${#FEATURES[@]}" -gt 0 ] && [ -n "${FEATURES[0]}" ]; then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then sel="$sel,${FEATURES[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features ${sel#,}")
  done
fi
printf '   declared non-default features: %s\n' "${FEATURES[*]:-<none>}"
printf '   combination: %s\n' "${COMBOS[@]/#/[cargo] }"

# ---------------------------------------------------------------------------
# Phase A.2 — build the C ground truth (executable via CMake + shared library).
# ---------------------------------------------------------------------------
step "Building the C ground truth"
(mkdir -p c_src/build && cd c_src/build &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build . >/dev/null) && ok "cmake --build (c_src/build/driver)" ||
  bad "cmake build"

mkdir -p target/verify
gcc -shared -fPIC -o target/verify/libdriver_c.so c_src/src/main.c &&
  ok "gcc -shared -fPIC -> target/verify/libdriver_c.so" || bad "gcc -shared"

# ---------------------------------------------------------------------------
# Phases B-D for every combination.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  step "cargo check ${label}"
  # shellcheck disable=SC2086
  if timeout 600 cargo check $CARGO_FLAGS --all-targets $combo >target/verify/check.log 2>&1; then
    ok "cargo check --all-targets ${label}"
  else
    bad "cargo check --all-targets ${label}"
    tail -30 target/verify/check.log
  fi

  step "cargo build (lib + bin + example) ${label}"
  # shellcheck disable=SC2086
  if timeout 600 cargo build $CARGO_FLAGS --lib --bins --examples $combo \
    >target/verify/build.log 2>&1; then
    ok "cargo build ${label}"
  else
    bad "cargo build ${label}"
    tail -30 target/verify/build.log
  fi

  step "Symbol parity (nm -D) ${label}"
  diff <(nm -D --defined-only target/verify/libdriver_c.so |
    awk '$2 ~ /^[TDBR]$/ {print $NF}' | sort) \
    <(nm -D --defined-only target/debug/libdriver.so |
      awk '$2 ~ /^[TDBR]$/ {print $NF}' | sort) >target/verify/symdiff.txt
  if [ -s target/verify/symdiff.txt ]; then
    bad "symbol diff is not empty:"
    cat target/verify/symdiff.txt
  else
    ok "every C .so symbol is exported by the Rust .so (and no extras)"
  fi

  step "Differential tests (Phases B + C + D) ${label}"
  # shellcheck disable=SC2086
  if DIFFTEST_CARGO_FEATURE_ARGS="$combo" timeout 600 \
    cargo test $CARGO_FLAGS $combo -- --include-ignored \
    >target/verify/test.log 2>&1; then
    ok "cargo test ${label}"
    grep -E "^(test result|in-process differential result)" target/verify/test.log |
      sed 's/^/      /'
  else
    bad "cargo test ${label}"
    tail -60 target/verify/test.log
  fi
done

# ---------------------------------------------------------------------------
# The shipped artifact: release profile (`panic = "abort"`).
# ---------------------------------------------------------------------------
step "Release profile (panic = abort)"
if timeout 600 cargo build $CARGO_FLAGS --release --lib --bins --examples \
  >target/verify/release-build.log 2>&1; then
  ok "cargo build --release"
else
  bad "cargo build --release"
  tail -30 target/verify/release-build.log
fi
diff <(nm -D --defined-only target/verify/libdriver_c.so |
  awk '$2 ~ /^[TDBR]$/ {print $NF}' | sort) \
  <(nm -D --defined-only target/release/libdriver.so |
    awk '$2 ~ /^[TDBR]$/ {print $NF}' | sort) >target/verify/symdiff-release.txt
if [ -s target/verify/symdiff-release.txt ]; then
  bad "release symbol diff is not empty:"
  cat target/verify/symdiff-release.txt
else
  ok "release .so symbol parity"
fi
if timeout 600 cargo test $CARGO_FLAGS --release >target/verify/test-release.log 2>&1; then
  ok "cargo test --release"
  grep -E "^(test result|in-process differential result)" target/verify/test-release.log |
    sed 's/^/      /'
else
  bad "cargo test --release"
  tail -60 target/verify/test-release.log
fi

step "Summary"
if [ "$FAILED" -eq 0 ]; then
  printf '   \033[32mALL CHECKS PASSED\033[0m\n'
  exit 0
fi
printf '   \033[31m%d CHECK(S) FAILED\033[0m\n' "$FAILED"
exit 1
