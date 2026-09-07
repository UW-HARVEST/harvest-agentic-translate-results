#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, diff exported symbols, and run the
# full differential suite under every declared feature combination.
set -uo pipefail
cd "$(dirname "$0")"

fail=0
note() { printf '%s\n' "$*"; }
bad()  { printf 'FAIL: %s\n' "$*"; fail=1; }

note "=== rebuild C shared library ==="
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO=$(ls ../c_src/build/*.so | head -1)
note "C  .so: $C_SO"

note "=== rebuild Rust cdylib ==="
timeout 600 cargo build --release >/dev/null 2>&1 || { bad "cargo build"; exit 1; }
R_SO=target/release/libarity_lib.so
note "Rust .so: $R_SO"

note ""
note "=== symbol parity (nm -D) ==="
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/pd_c.txt
nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/pd_r.txt
missing=$(comm -23 /tmp/pd_c.txt /tmp/pd_r.txt)
note "C exports $(wc -l < /tmp/pd_c.txt) symbols; Rust exports $(wc -l < /tmp/pd_r.txt)."
if [ -n "$missing" ]; then
  bad "symbols exported by C but MISSING from Rust:"; printf '  %s\n' $missing
else
  note "OK: symbol diff is EMPTY (0 C symbols missing from Rust)."
fi

note ""
note "=== unresolved imports in Rust .so ==="
# Authoritative check: `ldd -r` performs data+function relocation resolution and
# reports anything that cannot be satisfied by the recorded dependencies. This
# is stronger and less error-prone than regex-filtering `nm -D --undefined-only`
# (whose output legitimately contains libc, libgcc unwinder `_Unwind_*` and
# `_ITM_*`/`__gmon_start__` weak toolchain symbols).
ldd_out=$(ldd -r "$R_SO" 2>&1)
unresolved=$(printf '%s\n' "$ldd_out" | grep -E 'undefined symbol|not found' || true)
if [ -n "$unresolved" ]; then
  bad "Rust .so has unresolved imports:"; printf '  %s\n' "$unresolved"
else
  n_undef=$(nm -D --undefined-only "$R_SO" | wc -l)
  note "OK: 0 unresolved imports ($n_undef undefined entries, all satisfied by libc/libgcc)."
fi
# The C .so must likewise be clean, so the comparison is apples-to-apples.
c_unresolved=$(ldd -r "$C_SO" 2>&1 | grep -E 'undefined symbol|not found' || true)
if [ -n "$c_unresolved" ]; then
  bad "C .so has unresolved imports (unexpected):"; printf '  %s\n' "$c_unresolved"
else
  note "OK: C .so also has 0 unresolved imports."
fi
# Every libc function the C depends on must be reachable from Rust too.
note "C imports: $(nm -D --undefined-only "$C_SO" | awk '{print $NF}' | grep -E '^(malloc|free|memmove|strlen)' | tr '\n' ' ')"

note ""
note "=== binary/driver target check ==="
if grep -q 'add_executable' ../c_src/CMakeLists.txt 2>/dev/null \
   || [ -f src/main.rs ] || grep -q '^\[\[bin\]\]' Cargo.toml; then
  bad "an executable target exists -- stdout comparison required but not implemented"
else
  note "OK: no executable target on either side (library only); stdout comparison N/A."
fi

note ""
note "=== feature combinations ==="
combos=$(python3 - <<'PY'
import re
src = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip()
            if n != 'default':
                names.append(n)
print(' '.join(names))
PY
)
if [ -z "$combos" ]; then
  note "No [features] declared -> the only combination is the default/empty set."
  runs=("default:" "no-default-features:--no-default-features")
else
  runs=("default:" "no-default-features:--no-default-features")
  for f in $combos; do runs+=("$f:--no-default-features --features $f"); done
  runs+=("all-features:--all-features")
fi

for entry in "${runs[@]}"; do
  label="${entry%%:*}"; flags="${entry#*:}"
  note ""
  note "--- combo [$label] cargo test --release $flags ---"
  # shellcheck disable=SC2086
  out=$(timeout 600 cargo test --release $flags 2>&1)
  if [ $? -ne 0 ]; then
    bad "combo [$label] test run failed"
    printf '%s\n' "$out" | grep -E 'panicked|FAILED|test result|error' | head -30
  else
    printf '%s\n' "$out" | grep -E 'Running|test result'
  fi
done

note ""
if [ $fail -eq 0 ]; then
  note "PHASE D: ALL CHECKS PASSED"
else
  note "PHASE D: FAILURES PRESENT"
fi
exit $fail
