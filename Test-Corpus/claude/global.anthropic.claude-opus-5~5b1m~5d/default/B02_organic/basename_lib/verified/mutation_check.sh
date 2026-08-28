#!/usr/bin/env bash
# Meta-test: proves the differential suite is DISCRIMINATING.
#
# Each mutation injects a plausible translation bug into src/lib.rs, rebuilds
# the cdylib, and runs the suite. A mutation that still PASSES is a blind spot
# in the tests. src/lib.rs is always restored afterwards.
set -uo pipefail
cd "$(dirname "$0")"

BAK="$(mktemp)"
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; }
trap 'restore; rm -f "$BAK"' EXIT

BLIND=0

try_mutation() {
  local name="$1"; shift
  restore
  # shellcheck disable=SC2068
  perl -0pi -e "$1" src/lib.rs
  if cmp -s "$BAK" src/lib.rs; then
    printf '\033[31m  ?? %-42s MUTATION DID NOT APPLY\033[0m\n' "$name"
    BLIND=1; return
  fi

  if ! cargo build --offline >/dev/null 2>&1; then
    printf '\033[33m  -- %-42s did not compile (skipped)\033[0m\n' "$name"
    return
  fi

  local log; log="$(mktemp)"
  if cargo test --offline >"$log" 2>&1; then
    printf '\033[31m  !! %-42s SURVIVED -> tests have a BLIND SPOT\033[0m\n' "$name"
    BLIND=1
  else
    local first
    first="$(grep -oE '^test [a-z0-9_]+ \.\.\. FAILED' "$log" | head -3 | awk '{print $2}' | tr '\n' ' ')"
    printf '\033[32m  ok %-42s caught by: %s\033[0m\n' "$name" "${first:-<build/link failure>}"
  fi
  rm -f "$log"
}

echo "Mutation testing the differential suite:"

try_mutation "last-match -> first-match (rposition)" \
  's/\.rposition\(/.position(/'

try_mutation "ternary arm flipped (i1 > i2 -> i1 < i2)" \
  's/if i1 > i2 \{ i1 \} else \{ i2 \}/if i1 < i2 { i1 } else { i2 }/'

try_mutation "ternary arm dropped (always take slash)" \
  's/let last = if i1 > i2 \{ i1 \} else \{ i2 \};/let last = i1;/'

try_mutation "no-separator case returns NULL" \
  's/\(None, None\) => path,/(None, None) => core::ptr::null_mut(),/'

try_mutation "off-by-one: +1 dropped in both-present arm" \
  's/path\.add\(last \+ 1\)/path.add(last)/'

try_mutation "off-by-one: +1 dropped in slash-only arm" \
  's/\(Some\(i1\), None\) => unsafe \{ path\.add\(i1 \+ 1\) \}/(Some(i1), None) => unsafe { path.add(i1) }/'

try_mutation "backslash treated as non-separator" \
  's/let s2 = unsafe \{[^;]*\};/let s2: Option<usize> = None;/'

try_mutation "slash treated as non-separator" \
  's/let s1 = unsafe \{[^;]*\};/let s1: Option<usize> = None;/'

try_mutation "0x80 wrongly matched as separator" \
  's/b == needle/b == needle || b == 0x80/'

try_mutation "search stops at first non-ASCII byte" \
  's/\}\.to_bytes\(\);/}.to_bytes(); let bytes = \&bytes[..bytes.iter().position(|\&b| b >= 0x80).unwrap_or(bytes.len())];/'

try_mutation "byte compare widened to signed char" \
  's/bytes\.iter\(\)\.rposition\(\|&b\| b == needle\)/bytes.iter().rposition(|\&b| (b as i8) == (needle as i8) || b == (needle | 0x80))/'

try_mutation "input buffer mutated (separator NUL-ed out)" \
  's/let s1 = unsafe \{ strrchr_index\(path, b.\/.\) \};/let s1 = unsafe { strrchr_index(path, b"\/"[0]) }; if let Some(i) = s1 { unsafe { *path.add(i) = 0 } }/'

try_mutation "#[no_mangle] export removed" \
  's/#\[unsafe\(no_mangle\)\]//'

restore
cargo build --offline >/dev/null 2>&1
cargo build --release --offline >/dev/null 2>&1

echo
if [ "$BLIND" -eq 0 ]; then
  echo -e "\033[32mEvery applied mutation was caught: the suite is discriminating.\033[0m"
else
  echo -e "\033[31mAt least one mutation survived or failed to apply.\033[0m"
fi
exit "$BLIND"
