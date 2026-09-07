#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination + every build profile.
#
# Usage: ./verify_all.sh        (run from translation/)
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CBUILD="$ROOT/c_src/build"
FAIL=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  PASS  %s\n' "$*"; }
bad()  { printf '  FAIL  %s\n' "$*"; FAIL=1; }

# --------------------------------------------------------------------------
step "Building the C shared library"
mkdir -p "$CBUILD"
( cd "$CBUILD" && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$(find "$CBUILD" -maxdepth 1 -name '*.so' | head -1)"
[ -n "$C_SO" ] || { bad "no C .so produced"; exit 1; }
ok "C .so: $C_SO"

# --------------------------------------------------------------------------
# Enumerate feature combinations declared in Cargo.toml. If there is no
# [features] section the only configuration is the default one.
step "Enumerating cargo feature combinations"
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' "$CRATE/Cargo.toml")

if [ -z "$FEATURES" ]; then
  echo "  no [features] declared -> configurations: default, --no-default-features"
  COMBOS=("__default__" "__nodefault__")
else
  echo "  features: $FEATURES"
  COMBOS=("__default__" "__nodefault__")
  # power set of the declared features
  feats=($FEATURES); n=${#feats[@]}
  for ((m=1; m<(1<<n); m++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      (( m & (1<<i) )) && combo="${combo:+$combo,}${feats[$i]}"
    done
    COMBOS+=("$combo")
  done
fi
printf '  %d configuration(s) to verify\n' "${#COMBOS[@]}"

# --------------------------------------------------------------------------
for profile in release debug; do
  PROF_FLAG=""; [ "$profile" = release ] && PROF_FLAG="--release"

  for combo in "${COMBOS[@]}"; do
    case "$combo" in
      __default__)   FLAGS=""; LABEL="default" ;;
      __nodefault__) FLAGS="--no-default-features"; LABEL="no-default-features" ;;
      *)             FLAGS="--no-default-features --features $combo"; LABEL="features=$combo" ;;
    esac

    step "profile=$profile  config=$LABEL"

    # shellcheck disable=SC2086
    if ! ( cd "$CRATE" && timeout 600 cargo build $PROF_FLAG $FLAGS >/dev/null 2>&1 ); then
      bad "cargo build ($profile, $LABEL)"; continue
    fi
    RUST_SO="$CRATE/target/$profile/libreverse_collide_lib.so"
    [ -f "$RUST_SO" ] || { bad "no Rust .so at $RUST_SO"; continue; }

    # ---- symbol parity -----------------------------------------------------
    nm -D --defined-only "$C_SO"    | awk '$2=="T"||$2=="B"||$2=="D"{print $3}' | sort > /tmp/vc.$$
    nm -D --defined-only "$RUST_SO" | awk '$2=="T"||$2=="B"||$2=="D"{print $3}' | sort > /tmp/vr.$$
    MISSING=$(comm -23 /tmp/vc.$$ /tmp/vr.$$)
    EXTRA=$(comm -13 /tmp/vc.$$ /tmp/vr.$$)
    if [ -n "$MISSING" ]; then
      bad "symbols missing from Rust .so ($profile, $LABEL):"; echo "$MISSING" | sed 's/^/        /'
    else
      ok "symbol parity: $(wc -l < /tmp/vc.$$) C symbols, 0 missing"
    fi
    [ -n "$EXTRA" ] && { echo "  note: Rust exports extra symbols:"; echo "$EXTRA" | sed 's/^/        /'; }

    # ---- undefined non-libc symbols ---------------------------------------
    # `ldd -r` reports symbols that cannot actually be resolved against the
    # library's declared dependencies; anything satisfied by libc/libm/libgcc is
    # resolved and therefore not reported.
    UNRESOLVED=$(ldd -r "$RUST_SO" 2>&1 | grep -i 'undefined symbol' || true)
    if [ -n "$UNRESOLVED" ]; then
      bad "unresolvable symbols in Rust .so ($profile, $LABEL):"
      echo "$UNRESOLVED" | sed 's/^/        /'
    else
      ok "no unresolvable symbols (ldd -r clean)"
    fi
    # Additionally: no c2*/reverse_collide symbol may be merely *referenced*
    # rather than defined -- that would mean an untranslated module.
    LIBREF=$(nm -D --undefined-only "$RUST_SO" | awk '{print $2}' | sed 's/@.*//' \
             | grep -E '^(c2|reverse_collide)' | sort -u || true)
    if [ -n "$LIBREF" ]; then
      bad "Rust .so references library symbols it does not define:"
      echo "$LIBREF" | sed 's/^/        /'
    else
      ok "no library symbol is left undefined"
    fi
    rm -f /tmp/vc.$$ /tmp/vr.$$

    # ---- Phases B + C against THIS .so ------------------------------------
    # shellcheck disable=SC2086
    if ( cd "$CRATE" && RUST_SO_PATH="$RUST_SO" timeout 600 cargo test --release $FLAGS 2>&1 \
         | tee /tmp/vt.$$ | tail -3 ); then
      if grep -q "test result: FAILED" /tmp/vt.$$; then
        bad "tests failed ($profile, $LABEL)"
        grep -E "^(test .* FAILED|thread .* panicked)" /tmp/vt.$$ | head -20
      else
        PASSED=$(grep -oP '(?<=test result: ok\. )\d+' /tmp/vt.$$ | awk '{s+=$1} END{print s+0}')
        ok "all tests pass against $profile .so (${PASSED:-?} tests)"
      fi
    else
      bad "cargo test invocation failed ($profile, $LABEL)"
      grep -E "^(test .* FAILED|thread .* panicked|error)" /tmp/vt.$$ | head -20
    fi
    rm -f /tmp/vt.$$
  done
done

# --------------------------------------------------------------------------
step "Binary / driver executable check"
if grep -qE '^\[\[bin\]\]|^\s*\[\[bin\]\]' "$CRATE/Cargo.toml" \
   || grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  bad "a binary target exists but this script does not compare stdout"
else
  ok "no binary target in either build (c_src builds SHARED only; crate is cdylib only)"
fi

step "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
