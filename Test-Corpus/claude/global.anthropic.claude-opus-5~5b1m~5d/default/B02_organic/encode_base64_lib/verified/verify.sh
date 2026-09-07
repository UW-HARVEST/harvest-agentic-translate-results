#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination.
set -uo pipefail
cd "$(dirname "$0")"
ROOT=".."
C_SO="$ROOT/c_src/build/libdriver.so"
fail=0

echo "=== Build C .so ==="
( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null ) \
  || { echo "C build FAILED"; exit 1; }

echo "=== Enumerate feature combinations from Cargo.toml ==="
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{gsub(/ /,"");split($0,a,"=");if(a[1]!="default")print a[1]}' Cargo.toml)
if [ -z "$FEATS" ]; then
  echo "no [features] table -> single configuration (default)"
  COMBOS=("__default__")
else
  COMBOS=("__default__" "__none__")
  for f in $FEATS; do COMBOS+=("$f"); done
  COMBOS+=("$(echo $FEATS | tr ' ' ',')")
fi

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) ARGS=() ; label="default" ;;
    __none__)    ARGS=(--no-default-features) ; label="no-default-features" ;;
    *)           ARGS=(--no-default-features --features "$combo") ; label="features=$combo" ;;
  esac

  for profile in debug release; do
    PARGS=(); [ "$profile" = release ] && PARGS=(--release)
    echo
    echo "=== [$label / $profile] cargo build ==="
    cargo build --offline "${PARGS[@]}" "${ARGS[@]}" 2>&1 | tail -3 || fail=1

    RS_SO="target/$profile/libdriver.so"
    echo "--- symbol parity ($RS_SO) ---"
    nm -D --defined-only "$C_SO"  | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort -u > $HARVEST_WORKDIR/harvest-agent-tLFj47/tmp/claude_c_syms.txt
    nm -D --defined-only "$RS_SO" | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort -u > $HARVEST_WORKDIR/harvest-agent-tLFj47/tmp/claude_r_syms.txt
    missing=$(comm -23 $HARVEST_WORKDIR/harvest-agent-tLFj47/tmp/claude_c_syms.txt $HARVEST_WORKDIR/harvest-agent-tLFj47/tmp/claude_r_syms.txt)
    if [ -n "$missing" ]; then
      echo "MISSING FROM RUST .so:"; echo "$missing"; fail=1
    else
      echo "OK: 0 missing symbols ($(wc -l < $HARVEST_WORKDIR/harvest-agent-tLFj47/tmp/claude_c_syms.txt) C symbols all present)"
    fi

    echo "--- non-libc undefined symbols in Rust .so ---"
    und=$(nm -D --undefined-only "$RS_SO" | awk '{print $NF}' \
          | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__cxa\|^__gmon\|^__tls_get_addr\|^_Unwind' || true)
    if [ -n "$und" ]; then echo "UNRESOLVED: $und"; fail=1; else echo "OK: none"; fi

    echo "=== [$label / $profile] cargo test ==="
    cargo test --offline "${PARGS[@]}" "${ARGS[@]}" 2>&1 | grep -E '^test result:|FAILED|panicked' || fail=1
  done
done

echo
echo "=== binary target check ==="
if grep -q '^\[\[bin\]\]' Cargo.toml || [ -f src/main.rs ]; then
  echo "binary target present -- stdout comparison required"; fail=1
else
  echo "OK: no [[bin]] and no src/main.rs -> project builds no driver binary; stdout gate N/A"
fi

echo
[ "$fail" -eq 0 ] && echo "ALL PHASE D CHECKS PASSED" || echo "PHASE D FAILURES PRESENT"
exit $fail
