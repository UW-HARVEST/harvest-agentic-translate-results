#!/usr/bin/env bash
# Enumerates every valid feature combination declared in translation/Cargo.toml
# and runs check + build + differential tests for each, in both dev and release.
#
# The cdylib has to be built explicitly: `cargo test` alone does not emit
# target/<profile>/libpow.so for a crate-type = ["cdylib"] package, and the
# tests dlopen that file.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CRATE="$ROOT/translation"
CSRC="$ROOT/c_src"
TIMEOUT=600
rc=0

# ---- C reference library -----------------------------------------------------
echo "=== building C reference (default configuration) ==="
mkdir -p "$CSRC/build"
( cd "$CSRC/build" \
  && timeout $TIMEOUT cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout $TIMEOUT cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
test -f "$CSRC/build/libpow.so" || { echo "libpow.so missing"; exit 1; }
echo "ok: $CSRC/build/libpow.so"

# ---- enumerate features ------------------------------------------------------
# Read feature names out of the [features] table, skipping the "default" key.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/            { in_f=1; next }
    /^\[/                      { in_f=0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, p, "=");
      gsub(/[[:space:]]/, "", p[1]);
      if (p[1] != "default") print p[1];
    }
  ' "$CRATE/Cargo.toml"
)

n=${#FEATURES[@]}
echo
echo "=== feature enumeration ==="
if [ "$n" -eq 0 ]; then
  echo "Cargo.toml declares no [features]; the only configuration is the empty one."
else
  echo "declared features (${n}): ${FEATURES[*]}"
fi

# All 2^n subsets, expressed as comma-separated --features strings.
COMBOS=("")
for ((mask = 1; mask < (1 << n); mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if (((mask >> i) & 1)); then
      combo="${combo:+$combo,}${FEATURES[$i]}"
    fi
  done
  COMBOS+=("$combo")
done
echo "combinations to verify: ${#COMBOS[@]}"

# ---- matrix ------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    featflags=(--no-default-features)
    label="<none>"
  else
    featflags=(--no-default-features --features "$combo")
    label="$combo"
  fi

  for profile in dev release; do
    if [ "$profile" = release ]; then
      profflags=(--release)
    else
      profflags=()
    fi

    echo
    echo "--- features=[$label] profile=$profile ---"

    for step in check build test; do
      if ! timeout $TIMEOUT cargo "$step" "${profflags[@]}" "${featflags[@]}" \
             --manifest-path "$CRATE/Cargo.toml" > /tmp/pow_$step.log 2>&1; then
        echo "FAIL: cargo $step (features=[$label], $profile)"
        tail -n 30 /tmp/pow_$step.log
        rc=1
        continue 2
      fi
      echo "  ok: cargo $step"
    done
  done
done

# ---- exported symbol comparison ---------------------------------------------
echo
echo "=== exported dynamic symbols ==="
c_syms=$(nm -D --defined-only "$CSRC/build/libpow.so" | awk '{print $NF}' | sort -u)
for profile in debug release; do
  so="$CRATE/target/$profile/libpow.so"
  [ -f "$so" ] || continue
  rust_syms=$(nm -D --defined-only "$so" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$rust_syms"))
  if [ -n "$missing" ]; then
    echo "FAIL ($profile): symbols exported by C but not by Rust:"
    echo "$missing" | sed 's/^/  /'
    rc=1
  else
    echo "  ok ($profile): Rust exports every C symbol [$(echo "$c_syms" | tr '\n' ' ')]"
  fi
done

# ---- C optimisation-level cross-check ---------------------------------------
# The default CMake configuration sets no optimisation flags. Rebuilding the
# reference at -O0..-O3 confirms the errno-based error detection the Rust side
# mirrors is a property of the source, not of one particular C build.
#
# -Ofast / -ffast-math are deliberately excluded: they imply -fno-math-errno,
# which makes the *C* library stop detecting the errors at all (my_pow(-8, 1/3)
# returns -nan and prints nothing instead of returning -1). That is a different
# program, and the project's CMakeLists.txt does not enable it.
echo
echo "=== differential vs C rebuilt at -O0..-O3 ==="
for opt in O0 O1 O2 O3; do
  d="$(mktemp -d)"
  if ! gcc -shared -fPIC "-$opt" -I"$CSRC/include" -o "$d/libpow.so" "$CSRC/src/pow.c" -lm 2>/dev/null; then
    echo "  skip -$opt (compile failed)"; rm -rf "$d"; continue
  fi
  if POW_C_SO="$d/libpow.so" timeout $TIMEOUT cargo test --release \
       --manifest-path "$CRATE/Cargo.toml" > /tmp/pow_opt.log 2>&1; then
    echo "  ok: C -$opt"
  else
    echo "FAIL: differential against C -$opt"
    grep -E "^(test result:|thread )" /tmp/pow_opt.log | head -n 10
    rc=1
  fi
  rm -rf "$d"
done

echo
if [ $rc -eq 0 ]; then echo "ALL CONFIGURATIONS PASS"; else echo "FAILURES PRESENT"; fi
exit $rc
