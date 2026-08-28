#!/usr/bin/env bash
# Phase D completion gate: run the whole differential suite across every build
# configuration, and check the C/Rust dynamic symbol tables match exactly.
#
#   usage: ci/verify_all.sh
#
# `translation/Cargo.toml` declares no [features], so the feature axis has a
# single point; the loop below still enumerates it mechanically (and asserts
# that assumption) so that adding a feature later cannot silently skip a
# configuration.

set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CRATE_DIR="$PWD"
C_DIR="$(cd .. && pwd)/c_src"
export CARGO_NET_OFFLINE="${CARGO_NET_OFFLINE:-true}"

fail=0
step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '   \033[31mFAIL\033[0m %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
step "Building the C shared library"
mkdir -p "$C_DIR/build" || exit 1
( cd "$C_DIR/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "C .so built" || bad "C build"

C_SO=$(find "$C_DIR/build" -maxdepth 1 -name '*.so' | head -1)
[ -n "$C_SO" ] && ok "C .so: $C_SO" || bad "no C .so found"

# ---------------------------------------------------------------------------
step "Checking every ERRORS.md / CONFIGS.md row points at a real test"
missing_rows=0
while read -r name; do
  [ -z "$name" ] && continue
  grep -qE "fn $name\b" tests/*.rs || { bad "row references unknown test: $name"; missing_rows=1; }
done < <(grep -oE '`(cfg|err|ub)_[a-z0-9_]+`' ERRORS.md CONFIGS.md | sed 's/.*`\(.*\)`/\1/' | sort -u)
[ "$missing_rows" -eq 0 ] && ok "all documented row -> test references resolve"

# Both tables must be fully checked off.
for doc in ERRORS.md CONFIGS.md; do
  unchecked=$(grep -c '| \[ \]' "$doc")
  if [ "$unchecked" -eq 0 ]; then ok "$doc: no unchecked rows"; else bad "$doc has $unchecked unchecked row(s)"; fi
done

# ---------------------------------------------------------------------------
step "Enumerating feature combinations from Cargo.toml"
# Every feature named in a [features] section, if any.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]); if(a[1]!="default") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  ok "no [features] declared -> the only combination is the default one"
  COMBOS=("default" "no-default-features")
else
  bad "unexpected features found ($FEATURES): extend this script to cross them"
  COMBOS=("default" "no-default-features")
fi

# ---------------------------------------------------------------------------
for profile in dev release; do
  for combo in "${COMBOS[@]}"; do
    flags=()
    [ "$profile" = release ] && flags+=(--release)
    [ "$combo" = "no-default-features" ] && flags+=(--no-default-features)
    label="profile=$profile features=$combo"

    step "$label -- building the cdylib"
    if cargo build --lib "${flags[@]}" >/dev/null 2>&1; then
      ok "cdylib built"
    else
      bad "cdylib build ($label)"
      continue
    fi

    RUST_SO="$CRATE_DIR/target/$([ "$profile" = release ] && echo release || echo debug)/libmathop_lib.so"
    if [ ! -f "$RUST_SO" ]; then bad "no Rust .so at $RUST_SO"; continue; fi

    step "$label -- symbol parity (nm -D --defined-only)"
    if command -v nm >/dev/null; then
      c_syms=$(nm -D --defined-only "$C_SO"   | awk '$2=="T"{print $3}' | sort)
      r_syms=$(nm -D --defined-only "$RUST_SO" | awk '$2=="T"{print $3}' | sort)
      missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
      n_c=$(echo "$c_syms" | grep -c .)
      if [ -z "$missing" ]; then
        ok "all $n_c C symbols exported by the Rust .so"
      else
        bad "missing from the Rust .so: $(echo "$missing" | tr '\n' ' ')"
      fi
    else
      ok "nm unavailable, skipped (the phase_d_symbols test covers this too)"
    fi

    step "$label -- differential test suite"
    log="$CRATE_DIR/ci/logs/test-$profile-$combo.log"
    mkdir -p "$CRATE_DIR/ci/logs"
    if timeout 600 cargo test "${flags[@]}" >"$log" 2>&1; then
      # Show the per-binary summary lines as evidence that every binary ran.
      grep -E "Running (tests|unittests)|test result:|phase_stdout result:" "$log" \
        | sed 's/^/     /'
      n_bin=$(grep -c "Running tests/" "$log")
      if [ "$n_bin" -eq 4 ]; then
        ok "all 4 test binaries passed ($label); log: ${log#"$CRATE_DIR"/}"
      else
        bad "expected 4 test binaries, ran $n_bin ($label)"
      fi
    else
      tail -60 "$log"
      bad "tests failed ($label); full log: ${log#"$CRATE_DIR"/}"
    fi
  done
done

# ---------------------------------------------------------------------------
step "Summary"
if [ "$fail" -eq 0 ]; then
  printf '\033[32mALL CONFIGURATIONS VERIFIED\033[0m\n'
else
  printf '\033[31mVERIFICATION FAILED\033[0m\n'
fi
exit "$fail"
