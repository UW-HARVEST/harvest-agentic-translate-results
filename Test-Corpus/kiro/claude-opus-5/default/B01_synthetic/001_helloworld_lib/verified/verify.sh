#!/usr/bin/env bash
# Full verification sweep: builds both libraries, checks symbol parity, and runs
# the differential suite across every feature combination and both Rust build
# profiles. Nothing here is manual or per-configuration by hand.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CSRC="$ROOT/c_src"
FAIL=0

say() { printf '\n=== %s ===\n' "$*"; }
ok()  { printf '  OK    %s\n' "$*"; }
bad() { printf '  FAIL  %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
say "Build C shared library"
mkdir -p "$CSRC/build"
( cd "$CSRC/build" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) \
  && ok "libhello.so (C)" || bad "C build"
C_SO="$CSRC/build/libhello.so"
[ -f "$C_SO" ] || bad "missing $C_SO"

# ---------------------------------------------------------------------------
say "Enumerate feature combinations from Cargo.toml"
# Every declared feature (the [features] section keys). Empty => single config.
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
     sub(/[[:space:]]*=.*/,""); print
  }' "$CRATE/Cargo.toml")

if [ -z "$FEATURES" ]; then
  echo "  no [features] declared -> exactly one configuration"
  # The two ways an external consumer can build it; both must pass.
  COMBOS=("--no-default-features" "")
else
  echo "  features: $FEATURES"
  # Power set of the declared features.
  mapfile -t FARR <<<"$FEATURES"
  n=${#FARR[@]}
  COMBOS=()
  for ((m=0; m<(1<<n); m++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      (( m & (1<<i) )) && sel="${sel:+$sel,}${FARR[$i]}"
    done
    COMBOS+=("--no-default-features${sel:+ --features $sel}")
  done
  COMBOS+=("")   # plus the plain default build
fi
printf '  %d combination(s) to sweep\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
symbol_parity () {
  local rust_so="$1" label="$2"
  local c_syms rust_syms missing extra
  c_syms=$(nm -D --defined-only --format=posix "$C_SO"   | awk '{print $1}' | sort -u)
  rust_syms=$(nm -D --defined-only --format=posix "$rust_so" | awk '{print $1}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$rust_syms"))
  extra=$(comm -13 <(echo "$c_syms") <(echo "$rust_syms"))
  if [ -z "$missing" ]; then
    ok "symbol parity [$label]: 0 missing ($(echo "$c_syms" | wc -l) C symbol(s) all present)"
  else
    bad "symbol parity [$label]: MISSING from Rust .so: $(echo "$missing" | tr '\n' ' ')"
  fi
  [ -n "$extra" ] && echo "  note  extra Rust-only exports [$label]: $(echo "$extra" | tr '\n' ' ')"
  # Nothing may be left unresolved at load time either.
  if ldd -r "$rust_so" 2>&1 | grep -qiE 'undefined|not found'; then
    bad "ldd -r [$label]: unresolved symbols"
  else
    ok "ldd -r [$label]: all symbols resolve"
  fi
}

# ---------------------------------------------------------------------------
cd "$CRATE"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  say "Configuration: $label"

  # shellcheck disable=SC2086
  if timeout 600 cargo check $combo >/dev/null 2>&1; then ok "cargo check"; else bad "cargo check [$label]"; fi

  for profile in release debug; do
    relflag=""; [ "$profile" = release ] && relflag="--release"
    # shellcheck disable=SC2086
    if timeout 600 cargo build $relflag $combo >/dev/null 2>&1; then
      ok "cargo build ($profile)"
    else
      bad "cargo build $profile [$label]"; continue
    fi

    RUST_SO="$CRATE/target/$profile/libhello.so"
    [ -f "$RUST_SO" ] || { bad "missing $RUST_SO"; continue; }
    symbol_parity "$RUST_SO" "$label/$profile"

    # Run the differential suite against THIS .so.
    # shellcheck disable=SC2086
    res=$(HELLO_C_SO="$C_SO" HELLO_RUST_SO="$RUST_SO" \
          timeout 600 cargo test $combo -- --test-threads=1 2>&1 \
          | grep -E '^test result:' | tail -1)
    if echo "$res" | grep -q 'FAILED'; then
      bad "differential suite [$label/$profile]: $res"
      HELLO_C_SO="$C_SO" HELLO_RUST_SO="$RUST_SO" \
        timeout 600 cargo test $combo -- --test-threads=1 2>&1 \
        | grep -E '^test .* FAILED' | sed 's/^/        /'
    else
      ok "differential suite [$label/$profile]: $res"
    fi
  done
done

# ---------------------------------------------------------------------------
say "Driver binaries"
if grep -q add_executable "$CSRC/CMakeLists.txt" 2>/dev/null; then
  bad "C declares add_executable but no binary comparison is implemented"
else
  ok "C builds no executable (add_library only) -> no stdout comparison to make"
fi
if grep -q '^\[\[bin\]\]' "$CRATE/Cargo.toml" || [ -f "$CRATE/src/main.rs" ] || [ -d "$CRATE/src/bin" ]; then
  bad "Rust declares a binary but no binary comparison is implemented"
else
  ok "Rust builds no binary ([[bin]]/src/main.rs/src/bin all absent)"
fi

say "No stubs in the translation"
if grep -rnE 'unimplemented!|todo!|panic!\("not' "$CRATE/src"; then
  bad "translation contains stubbed functionality"
else
  ok "no unimplemented!/todo! in translation/src"
fi

say "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit "$FAIL"
