#!/usr/bin/env bash
# Full differential verification: builds both shared objects, then runs every
# phase under every Cargo feature combination.
#
# Usage: translation/verify.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
FAILED=0

echo "=== [1/4] build the C shared library ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
echo "C  .so: $C_SO"

echo
echo "=== [2/4] enumerate feature combinations ==="
# Every declared feature, then the powerset. `translation` declares none, so the
# powerset is just the empty set -> the default build.
FEATS=$(cd "$ROOT/translation" && awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[a-zA-Z0-9_-]+[[:space:]]*=/ { split($0,a,"="); gsub(/[[:space:]]/,"",a[1]);
    if (a[1] != "default") print a[1] }
' Cargo.toml | sort -u | tr '\n' ' ')
FEATS="$(echo "$FEATS" | xargs || true)"

COMBOS=()
if [[ -z "$FEATS" ]]; then
  echo "no [features] declared -> combinations: default, --no-default-features"
  COMBOS+=("")                       # default
  COMBOS+=("--no-default-features")  # explicitly empty
else
  read -r -a FARR <<< "$FEATS"
  n=${#FARR[@]}
  echo "features: ${FARR[*]}  -> $((1 << n)) combinations"
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      (( mask & (1 << i) )) && sel="${sel:+$sel,}${FARR[$i]}"
    done
    COMBOS+=("--no-default-features${sel:+ --features $sel}")
  done
  COMBOS+=("")  # plus the plain default build
fi

echo
echo "=== [3/4] cargo check per combination ==="
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  if ( cd "$ROOT/translation" && timeout 600 cargo check --quiet $combo >/dev/null 2>&1 ); then
    echo "  check  OK    $label"
  else
    echo "  check  FAIL  $label"; FAILED=1
  fi
done

echo
echo "=== [4/4] build + test per combination (phases A-D) ==="
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo "--- combination: $label ---"
  ( cd "$ROOT/translation" && timeout 600 cargo build --release --quiet $combo ) \
    || { echo "  build FAIL $label"; FAILED=1; continue; }

  RS_SO="$ROOT/translation/target/release/libcharinbuf_lib.so"
  echo -n "  symbol diff: "
  if diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
          <(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort) >/dev/null; then
    echo "empty (OK)"
  else
    echo "NON-EMPTY (FAIL)"
    diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
         <(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort)
    FAILED=1
  fi

  if ( cd "$ROOT/translation" \
       && C_SO="$C_SO" RUST_SO="$RS_SO" \
          timeout 600 cargo test $combo -- --test-threads=1 2>&1 \
          | tee /tmp/verify-$$.log | grep -E "^test result:" ); then
    grep -qE "^test result: FAILED" /tmp/verify-$$.log && FAILED=1
  else
    echo "  tests FAIL $label"; FAILED=1
  fi
  rm -f /tmp/verify-$$.log
done

echo
if [[ $FAILED -eq 0 ]]; then
  echo "ALL PHASES PASSED under all ${#COMBOS[@]} feature combination(s)."
else
  echo "VERIFICATION FAILED"
fi
exit $FAILED
