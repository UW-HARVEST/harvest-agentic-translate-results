#!/usr/bin/env bash
# Full verification driver: builds the C .so and the Rust .so, diffs the
# exported symbol sets (Phase D), and runs the differential test suites
# (Phase B + Phase C) under every feature combination.
set -u
cd "$(dirname "$0")"
ROOT="$(pwd)"
C_SO="$ROOT/../c_src/build/libdriver.so"
RS_SO="$ROOT/target/release/libdriver.so"
rc=0

echo "=== 1. Build the C shared library ==========================="
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
echo "ok: $C_SO"

# Enumerate feature combinations. This crate declares no [features], so the
# powerset is just the default build; the loop below is written generically so
# it keeps working if features are added later.
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' Cargo.toml)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS+=("default::")
  COMBOS+=("no-default-features:--no-default-features:")
  COMBOS+=("all-features:--all-features:")
else
  n=${#FEATURES[@]}
  for ((mask=0; mask<(1<<n); mask++)); do
    sel=()
    for ((i=0; i<n; i++)); do (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}"); done
    joined=$(IFS=,; echo "${sel[*]:-}")
    COMBOS+=("nodefault[$joined]:--no-default-features --features=$joined:")
  done
  COMBOS+=("default::")
  COMBOS+=("all-features:--all-features:")
fi

for entry in "${COMBOS[@]}"; do
  name="${entry%%:*}"; rest="${entry#*:}"; flags="${rest%%:*}"
  echo
  echo "=== 2. Feature combo: $name  (cargo flags: ${flags:-<none>}) ==="

  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --release --offline $flags >/dev/null 2>&1; then
    echo "  Rust release build FAILED for combo $name"; rc=1; continue
  fi

  echo "  -- symbol parity (nm -D) --"
  c_syms=$(nm -D --defined-only "$C_SO"  | awk '$2=="T"{print $3}' | sort -u)
  r_syms=$(nm -D --defined-only "$RS_SO" | awk '$2=="T"{print $3}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  if [ -n "$missing" ]; then
    echo "  MISSING from Rust .so:"; echo "$missing" | sed 's/^/    /'; rc=1
  else
    echo "  ok: all $(echo "$c_syms" | wc -l) C symbols exported by Rust: $(echo $c_syms | tr '\n' ' ')"
  fi
  undef_bad=$(nm -D --undefined-only "$RS_SO" \
              | awk '{print $NF}' \
              | grep -v -E '@GLIBC_|@GCC_|^_ITM_|^__gmon_start__$|^_Unwind_' || true)
  if [ -n "$undef_bad" ]; then
    echo "  UNDEFINED non-libc symbols in Rust .so:"; echo "$undef_bad" | sed 's/^/    /'; rc=1
  else
    echo "  ok: 0 undefined non-libc symbols"
  fi
  if grep -qE 'unimplemented!|todo!|unreachable!\(\)' src/lib.rs; then
    echo "  STUB MACRO found in src/lib.rs"; rc=1
  else
    echo "  ok: no stubs in src/lib.rs"
  fi

  echo "  -- differential tests (Phase B + Phase C) --"
  # shellcheck disable=SC2086
  if timeout 600 cargo test --offline $flags 2>&1 | grep -E "^test result"; then :; fi
  # shellcheck disable=SC2086
  if ! timeout 600 cargo test --offline $flags >/dev/null 2>&1; then
    echo "  TESTS FAILED for combo $name"; rc=1
  else
    echo "  ok: all tests pass for combo $name"
  fi
done

echo
if [ "$rc" -eq 0 ]; then
  echo "RESULT: PASS — symbol parity + all differential tests green in every combo"
else
  echo "RESULT: FAIL — see above"
fi
exit "$rc"
