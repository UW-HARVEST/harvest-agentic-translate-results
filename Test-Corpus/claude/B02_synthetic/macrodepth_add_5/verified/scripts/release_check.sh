#!/bin/bash
# Release-profile (panic = "abort", optimised) sanity check: every combination
# must still build, and the release executable must match the C byte-for-byte.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT" || exit 1
fail=0
ARGS=("3 4" "0 0" "-5 9" "2147483647 1" "-2147483648 -1" "46341 46341" "abc x" "2147483648 -2147483649" "" "1" "  -34  " "9223372036854775808 -9223372036854775809")
for c in $(./scripts/combos.sh); do
  op="${c%,*}"; r="${c#*,}"
  cargo build --offline --quiet --release --no-default-features --features "$op,$r" \
    || { echo "FAIL release build $op,$r"; fail=1; continue; }
  d="artifacts/${op}_${r}/relbin"; mkdir -p "$d"; cp target/release/driver "$d/driver"
  nmc="$(nm -D --defined-only "artifacts/${op}_${r}/libcdriver.so" | awk '{print $2,$3}' | sort)"
  nmr="$(nm -D --defined-only target/release/libdriver.so | awk '{print $2,$3}' | sort)"
  [ "$nmc" = "$nmr" ] || { echo "FAIL release symbols $op,$r"; diff <(printf '%s\n' "$nmc") <(printf '%s\n' "$nmr"); fail=1; }
  for a in "${ARGS[@]}"; do
    co="$(cd "artifacts/${op}_${r}/cbin" && ./driver $a 2>&1; echo rc=$?)"
    ro="$(cd "$d" && ./driver $a 2>&1; echo rc=$?)"
    [ "$co" = "$ro" ] || { echo "FAIL release output $op,$r args='$a'"; diff <(printf '%s\n' "$co") <(printf '%s\n' "$ro"); fail=1; }
  done
  echo "OK release $op REPEAT=$r"
done
[ $fail -eq 0 ] && echo "RELEASE PROFILE: BUILDS, SYMBOLS AND OUTPUT ALL MATCH" || echo "RELEASE PROFILE FAILURES"
exit $fail
