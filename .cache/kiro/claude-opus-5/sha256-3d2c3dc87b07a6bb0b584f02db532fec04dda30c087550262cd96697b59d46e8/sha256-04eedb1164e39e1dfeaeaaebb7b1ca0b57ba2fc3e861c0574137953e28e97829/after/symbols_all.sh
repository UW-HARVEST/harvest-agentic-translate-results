#!/usr/bin/env bash
# Step 8 across every configuration: every dynamic symbol the C shared object
# exports must also be exported, under the same name, by the Rust cdylib.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT/translation"
fail=0
syms() { nm -D --defined-only "$1" | awk '{print $3}' | sort; }

for op in add sub mul; do
  for r in 0 1 2 3 4 5 6 7; do
    timeout 300 cargo build --release --no-default-features --features "$op,$r" >/dev/null 2>&1 || {
      echo "### BUILD FAIL $op/$r"; fail=1; continue; }
    cso="/tmp/cref/${op}_${r}/libmd.so"
    rso="target/release/libmacrodepth_add_5.so"
    missing=$(comm -23 <(syms "$cso") <(syms "$rso"))
    if [[ -n "$missing" ]]; then
      echo "### MISSING EXPORTS $op/$r: $(echo "$missing" | tr '\n' ' ')"
      fail=1
    else
      echo "ok  syms [$op,$r]  $(syms "$cso" | tr '\n' ' ')"
    fi
  done
done
[[ $fail -eq 0 ]] && echo "=== PASS (symbols) ==="
exit $fail
