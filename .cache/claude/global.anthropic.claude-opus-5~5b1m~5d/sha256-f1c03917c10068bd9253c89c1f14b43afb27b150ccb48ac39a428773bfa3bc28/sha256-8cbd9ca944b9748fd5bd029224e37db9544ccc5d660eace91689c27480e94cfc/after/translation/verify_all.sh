#!/usr/bin/env bash
# Full verification sweep: builds the C .so and every Rust configuration, then
# runs the whole differential suite against each Rust artifact.
#
# Usage:  ./verify_all.sh [extra cargo flags...]
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
CARGO_FLAGS=("--offline" "$@")
fail=0

step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  PASS  %s\n' "$*"; }
bad()  { printf '  FAIL  %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
step "Building the C shared library"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$root/c_src/build/libdriver.so"
ok "C .so -> $C_SO"

# ---------------------------------------------------------------------------
# Feature combinations.  `Cargo.toml` declares no [features] table, so the set
# of distinct configurations is derived mechanically and comes out as the single
# default build; the loop below still enumerates it so that adding a feature
# later is picked up automatically.
step "Enumerating feature combinations from Cargo.toml"
mapfile -t FEATURES < <(
  python3 - "$here/Cargo.toml" <<'PY'
import sys, re
txt = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip()
            if n != 'default':
                names.append(n)
print("__default__")
print("__none__")
import itertools
for r in range(1, len(names) + 1):
    for combo in itertools.combinations(names, r):
        print(",".join(combo))
if names:
    print("__all__")
PY
)
printf '  combinations: %s\n' "${FEATURES[*]}"

feature_flags() {
  case "$1" in
    __default__) : ;;
    __none__)    echo "--no-default-features" ;;
    __all__)     echo "--all-features" ;;
    *)           echo "--no-default-features --features $1" ;;
  esac
}

# ---------------------------------------------------------------------------
step "cargo check for every feature combination"
for f in "${FEATURES[@]}"; do
  # shellcheck disable=SC2046
  if cargo check "${CARGO_FLAGS[@]}" $(feature_flags "$f") >/dev/null 2>&1; then
    ok "cargo check [$f]"
  else
    bad "cargo check [$f]"
  fi
done

# ---------------------------------------------------------------------------
# Run the suite once per (feature combination x profile).  Both profiles matter:
# `release` turns on optimisation and `panic = "abort"`.
for f in "${FEATURES[@]}"; do
  for profile in dev release; do
    step "Suite: features=[$f] profile=$profile"
    prof_flag=""; prof_dir="debug"
    if [ "$profile" = release ]; then prof_flag="--release"; prof_dir="release"; fi

    # shellcheck disable=SC2046
    cargo build "${CARGO_FLAGS[@]}" $prof_flag $(feature_flags "$f") >/dev/null 2>&1 \
      || { bad "build features=[$f] profile=$profile"; continue; }

    RUST_SO="$here/target/$prof_dir/libdriver.so"
    [ -f "$RUST_SO" ] || { bad "missing $RUST_SO"; continue; }

    echo "  --- nm -D symbol diff (C minus Rust) ---"
    diff_out=$(comm -23 \
      <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u))
    if [ -z "$diff_out" ]; then
      ok "symbol parity (0 missing) features=[$f] profile=$profile"
    else
      bad "symbols missing from Rust .so: $(echo "$diff_out" | tr '\n' ' ')"
    fi

    # Tests always run in the dev profile (the harness is what we compile); the
    # library under test is pinned with CTORUST_RUST_SO.
    # shellcheck disable=SC2046
    if CTORUST_RUST_SO="$RUST_SO" cargo test "${CARGO_FLAGS[@]}" \
         $(feature_flags "$f") -- --test-threads=1 2>&1 | tail -n 25; then
      ok "differential suite features=[$f] profile=$profile"
    else
      bad "differential suite features=[$f] profile=$profile"
    fi
  done
done

# ---------------------------------------------------------------------------
step "Binary executables"
if grep -q 'add_executable' "$root/c_src/CMakeLists.txt" 2>/dev/null; then
  bad "c_src builds an executable but no stdout comparison is wired up"
else
  ok "no [[bin]]/add_executable in either project - nothing to compare"
fi

# ---------------------------------------------------------------------------
if [ "$fail" -eq 0 ]; then
  printf '\n########## ALL CONFIGURATIONS VERIFIED ##########\n'
else
  printf '\n########## FAILURES PRESENT ##########\n'
fi
exit "$fail"
