#!/usr/bin/env bash
# Full verification sweep: build both libraries, then run the differential test
# suite under every Cargo feature combination and both profiles.
#
# Usage: ./verify.sh            (from the `translation` directory)
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
FAILED=0
TIMEOUT=600

step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ ok ] %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
step "Build the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "c_src/build/libdriver.so" || bad "C build"

# ---------------------------------------------------------------------------
step "Enumerate feature combinations from Cargo.toml"
# Every named feature declared in [features], excluding the implicit "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE_DIR/Cargo.toml"
)

if [ "${#FEATURES[@]}" -eq 0 ]; then
  ok "no [features] declared -> exactly one configuration (default)"
  COMBOS=("__default__")
else
  ok "features: ${FEATURES[*]}"
  # Power set of the declared features, plus the default configuration.
  COMBOS=("__default__")
  n=${#FEATURES[@]}
  for (( mask=0; mask < (1<<n); mask++ )); do
    combo=""
    for (( b=0; b<n; b++ )); do
      if (( mask & (1<<b) )); then combo="${combo:+$combo,}${FEATURES[b]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
printf '  %s combination(s) to verify\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
run_combo() {
  local profile="$1" combo="$2" label flags=()
  if [ "$combo" = "__default__" ]; then
    label="default features"
  else
    label="--no-default-features --features '${combo}'"
    flags=(--no-default-features --features "$combo")
  fi
  [ "$profile" = "release" ] && flags+=(--release)

  # The cdylib under test must be rebuilt for this combination before the tests
  # load it, because the tests dlopen the artifact rather than linking it.
  if ! timeout "$TIMEOUT" cargo build "${flags[@]}" >/dev/null 2>&1; then
    bad "$profile / $label : cargo build"
    return
  fi
  if ! timeout "$TIMEOUT" cargo clippy "${flags[@]}" --all-targets \
        -- -D warnings >/dev/null 2>&1; then
    printf '  [warn] %s / %s : clippy reported findings (non-fatal)\n' "$profile" "$label"
  fi
  local out
  out=$(timeout "$TIMEOUT" cargo test "${flags[@]}" 2>&1)
  if printf '%s' "$out" | grep -q 'test result: FAILED'; then
    bad "$profile / $label"
    printf '%s\n' "$out" | grep -E '^test result|^    [a-z0-9_]+$|panicked at' | head -30 | sed 's/^/      /'
  else
    ok "$profile / $label : $(printf '%s' "$out" | grep -c '\.\.\. ok') tests passed"
  fi
}

for profile in release dev; do
  step "cargo test ($profile profile) across all feature combinations"
  for combo in "${COMBOS[@]}"; do
    run_combo "$profile" "$combo"
  done
done

# ---------------------------------------------------------------------------
step "Symbol parity (nm -D)"
C_SO="$ROOT/c_src/build/libdriver.so"
# Prefer the release artifact; fall back to debug.
R_SO="$CRATE_DIR/target/release/libdriver.so"
[ -f "$R_SO" ] || R_SO="$CRATE_DIR/target/debug/libdriver.so"

nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /dev/null
missing=$(comm -23 \
  <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u))
if [ -z "$missing" ]; then
  ok "0 symbols missing from the Rust .so"
else
  bad "missing from Rust .so: $(echo "$missing" | tr '\n' ' ')"
fi

extra=$(comm -13 \
  <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u))
[ -z "$extra" ] && ok "no extra exports" \
  || printf '  [info] extra exports in Rust .so: %s\n' "$(echo "$extra" | tr '\n' ' ')"

step "Undefined non-libc symbols in the Rust .so"
und=$(nm -D -u "$R_SO" | awk '{print $NF}' | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^$' || true)
[ -z "$und" ] && ok "none" || bad "unresolved: $(echo "$und" | tr '\n' ' ')"

# ---------------------------------------------------------------------------
step "Per-function instruction-stream comparison"
norm() {
  objdump -d --no-show-raw-insn "$1" \
    | awk -v f="$2" '$0 ~ "^0*[0-9a-f]+ <"f">:",/^$/' \
    | grep -vE 'int3|^$|>:' \
    | sed -E 's/^[[:space:]]*[0-9a-f]+:\t//; s/[[:space:]]*(#|<).*$//;
              s/(-?0x[0-9a-f]+)\(%rip\)/RIP/;
              s/(call|je|jmp)[[:space:]]+[0-9a-f]+/\1 TARGET/'
}
for f in printIntPtrLine bad good driver; do
  if diff <(norm "$C_SO" "$f") <(norm "$R_SO" "$f") >/dev/null; then
    ok "$f: identical ($(norm "$C_SO" "$f" | wc -l) instructions)"
  else
    bad "$f: instruction streams differ"
    diff <(norm "$C_SO" "$f") <(norm "$R_SO" "$f") | sed 's/^/      /'
  fi
done

# ---------------------------------------------------------------------------
step "ELF profile comparison"
for l in "$C_SO" "$R_SO"; do
  printf '  %-46s NEEDED=%s dynsym=%s BIND_NOW=%s\n' \
    "$(basename "$(dirname "$l")")/$(basename "$l")" \
    "$(readelf -d "$l" | grep -c '(NEEDED)')" \
    "$(readelf -sW --dyn-syms "$l" | grep -c ':')" \
    "$(readelf -d "$l" | grep -c 'BIND_NOW')"
done

# ---------------------------------------------------------------------------
printf '\n'
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "SOME CHECKS FAILED"
fi
exit "$FAILED"
