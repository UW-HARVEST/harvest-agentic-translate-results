#!/usr/bin/env bash
# Step 8, swept over every configuration: every dynamic symbol defined by the C
# shared object must also be defined by the Rust cdylib, under the same name.
# Checked for both the dev and release profiles.
set -u
cd "$(dirname "$0")/translation"

syms() { nm -D --defined-only "$1" | awk 'NF{print $NF}' | sort -u; }

fail=0
for profile in dev release; do
  case $profile in
    dev)     pflag=();          dir=target/debug ;;
    release) pflag=(--release); dir=target/release ;;
  esac
  for OP in add sub mul; do
    for R in 0 1 2 3 4 5 6 7; do
      if ! out=$(timeout 300 cargo build "${pflag[@]}" --no-default-features --features "$OP,$R" 2>&1); then
        echo "BUILD FAIL $profile $OP/$R"; printf '%s\n' "$out" | grep -E '^error' | head -5
        fail=1; continue
      fi
      cso="/tmp/cref/${OP}_${R}/libmdcore.so"
      rso="$dir/libdriver.so"
      missing=$(comm -23 <(syms "$cso") <(syms "$rso"))
      if [[ -n "$missing" ]]; then
        echo "MISSING EXPORTS $profile $OP/$R:"; printf '%s\n' "$missing" | sed 's/^/    /'
        fail=1
      fi
      # No C-style extras beyond the Rust runtime's own mangled symbols.
      extra=$(comm -13 <(syms "$cso") <(syms "$rso") \
              | grep -vE '^(_ZN|_R|rust_|__rust)' | grep -v '17h')
      if [[ -n "$extra" ]]; then
        echo "EXTRA EXPORTS $profile $OP/$R:"; printf '%s\n' "$extra" | sed 's/^/    /'
        fail=1
      fi
    done
  done
  echo "  $profile profile: 24 configurations checked"
done
[[ $fail -eq 0 ]] && echo "SYMBOL PARITY OK FOR ALL CONFIGURATIONS (dev + release)"
exit $fail
