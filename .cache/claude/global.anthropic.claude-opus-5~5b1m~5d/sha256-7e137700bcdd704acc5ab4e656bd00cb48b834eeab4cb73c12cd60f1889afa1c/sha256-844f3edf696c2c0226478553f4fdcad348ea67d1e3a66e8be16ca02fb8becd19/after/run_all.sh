#!/bin/bash
# Runs the full Phase B + C + D differential suite for EVERY valid feature
# combination (at most one OP feature x at most one REPEAT feature = 36 combos),
# each against the C artifacts built with the matching -DOP/-DREPEAT.
set -u
root="$(cd "$(dirname "$0")" && pwd)"
cd "$root/translation" || exit 1

filter="${1:-}"
fail=0
pass=0

run_combo() {
  local combo="$1" cop="$2" crep="$3" tag="$4"
  export C_SO="$root/cbuild/libcdriver_${cop}_${crep}.so"
  export C_BIN="$root/cbuild/driver_${cop}_${crep}"
  # The "no features" combination is compared against a C object compiled with
  # no -D flags at all, i.e. exercising the #ifndef defaults.
  if [ "$tag" = default ]; then
    C_SO="$root/cbuild/libcdriver_default.so"
    C_BIN="$root/cbuild/driver_default"
  fi
  local log
  log="$(mktemp "${TMPDIR:-/tmp}/combo.XXXXXX.log")"
  if cargo build --no-default-features --features "$combo" >"$log" 2>&1 \
     && timeout 600 cargo test --no-default-features --features "$combo" \
          ${filter:+"$filter"} -- --test-threads=1 >>"$log" 2>&1; then
    printf 'PASS  features=[%-8s] C=%s/%s\n' "$combo" "$cop" "$crep"
    pass=$((pass + 1))
  else
    printf 'FAIL  features=[%-8s] C=%s/%s\n' "$combo" "$cop" "$crep"
    tail -40 "$log"
    fail=$((fail + 1))
  fi
  rm -f "$log"
}

for op in "" add sub mul; do
  for rep in "" 0 1 2 3 4 5 6 7; do
    combo="$(echo "$op $rep" | tr ' ' ',' | sed 's/^,//; s/,$//')"
    cop="${op:-add}"
    crep="${rep:-5}"
    tag=explicit
    [ -z "$op" ] && [ -z "$rep" ] && tag=default
    run_combo "$combo" "$cop" "$crep" "$tag"
  done
done

echo "-----"
echo "combinations passed: $pass, failed: $fail"
exit $((fail > 0))
