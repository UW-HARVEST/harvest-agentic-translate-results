#!/usr/bin/env bash
# Phase D driver: symbol parity + every build configuration.
#
#   ./verify.sh
#
# Enumerates the feature combinations declared in Cargo.toml (there are none, so
# the set reduces to default / --no-default-features / --all-features), builds
# the Rust cdylib for each in both profiles, diffs `nm -D` against the C .so, and
# runs the full differential test suite against each resulting .so.
set -uo pipefail
cd "$(dirname "$0")"

TIMEOUT=${TIMEOUT:-600}
CSO=$(ls ../c_src/build/*.so | head -1)
fail=0
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

note() { printf '\n=== %s ===\n' "$*"; }

# --- feature combinations, extracted from Cargo.toml ------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"="); gsub(/ /,"",a[1]); if (a[1]!="default") print a[1]}' Cargo.toml
)
COMBOS=("" "--no-default-features" "--all-features")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
  note "declared features: ${FEATURES[*]}"
else
  note "Cargo.toml declares no [features]; 3 equivalent configurations"
fi

# --- cargo check for every combination --------------------------------------
for combo in "${COMBOS[@]}"; do
  note "cargo check ${combo:-<default>}"
  if ! timeout "$TIMEOUT" cargo check $combo >"$TMP/chk.log" 2>&1; then
    tail -20 "$TMP/chk.log"; fail=1
  else
    echo "ok"
  fi
done

# --- build + symbol diff + tests, per profile and combination ---------------
nm -D --defined-only "$CSO" | awk '{print $3}' | sort -u > "$TMP/c_syms.txt"
echo "C exports: $(wc -l < "$TMP/c_syms.txt")"

for profile in release debug; do
  flag=""; [ "$profile" = release ] && flag="--release"
  for combo in "${COMBOS[@]}"; do
    note "profile=$profile combo=${combo:-<default>}"

    if ! timeout "$TIMEOUT" cargo build $flag $combo >"$TMP/build.log" 2>&1; then
      tail -20 "$TMP/build.log"; fail=1; continue
    fi
    RSO="target/$profile/libsh_puts_lib.so"

    nm -D --defined-only "$RSO" | awk '{print $3}' | sort -u > "$TMP/r_syms.txt"
    missing=$(comm -23 "$TMP/c_syms.txt" "$TMP/r_syms.txt")
    if [ -n "$missing" ]; then
      echo "MISSING SYMBOLS:"; echo "$missing"; fail=1
    else
      echo "symbol diff: empty (all $(wc -l < "$TMP/c_syms.txt") C exports present)"
    fi

    undef=$(nm -D --undefined-only "$RSO" | awk '{print $2}' \
      | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^_Unwind_' || true)
    if [ -n "$undef" ]; then
      echo "UNRESOLVED NON-LIBC SYMBOLS:"; echo "$undef"; fail=1
    else
      echo "undefined: libc/libgcc only"
    fi

    if ! SHPUTS_RUST_SO="$PWD/$RSO" timeout "$TIMEOUT" \
         cargo test --release $combo >"$TMP/test.log" 2>&1; then
      grep -E 'test result|panicked|FAILED|Assertion' "$TMP/test.log" | head -30
      fail=1
    else
      grep -E 'test result' "$TMP/test.log"
    fi
  done
done

note "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASS"; else echo "FAILURES PRESENT"; fi
exit "$fail"
