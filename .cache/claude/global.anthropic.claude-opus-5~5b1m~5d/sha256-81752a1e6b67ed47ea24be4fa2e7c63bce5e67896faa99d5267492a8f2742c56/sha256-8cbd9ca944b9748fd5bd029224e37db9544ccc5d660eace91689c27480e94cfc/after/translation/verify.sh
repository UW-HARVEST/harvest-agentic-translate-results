#!/usr/bin/env bash
# Phase D driver: symbol parity + the full differential suite under every
# feature combination and both profiles.
set -uo pipefail
cd "$(dirname "$0")"
ROOT=".."
fail=0

echo "=== Building C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/lib*.so)
echo "C  : $C_SO"

# Every feature combination declared in Cargo.toml (plus the default set).
mapfile -t FEATS < <(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{print $1}' Cargo.toml)
echo "=== Feature combinations ==="
if [ ${#FEATS[@]} -eq 0 ]; then
  echo "Cargo.toml declares no [features]; the default set is the only combination."
  COMBOS=("")
else
  COMBOS=("")
  for f in "${FEATS[@]}"; do COMBOS+=("$f"); done
  COMBOS+=("$(IFS=,; echo "${FEATS[*]}")")
fi

for profile in debug release; do
  for combo in "${COMBOS[@]}"; do
    if [ -z "$combo" ]; then featargs=(); label="(default features)";
    else featargs=(--no-default-features --features "$combo"); label="--features $combo"; fi
    relargs=(); [ "$profile" = release ] && relargs=(--release)

    echo
    echo "=== $profile $label ==="
    cargo build --offline "${relargs[@]}" "${featargs[@]}" >/dev/null 2>&1 \
      || { echo "  cargo build FAILED"; fail=1; continue; }

    R_SO="target/$profile/libmatrixsum_lib.so"
    echo "--- symbol diff (C vs Rust) ---"
    strip_syms() { nm -D --defined-only "$1" | awk '{print $2, $3}' \
        | grep -Ev ' (_init|_fini|__bss_start|_edata|_end)$' | sort; }
    d=$(diff <(strip_syms "$C_SO") <(strip_syms "$R_SO"))
    if [ -n "$d" ]; then echo "  SYMBOL DIFF NOT EMPTY:"; echo "$d"; fail=1;
    else echo "  identical: $(strip_syms "$C_SO" | wc -l) symbols"; fi

    echo "--- unresolved (truly undefined) symbols in Rust .so ---"
    # `ldd -r` reports any import that cannot be satisfied by the system
    # libraries, i.e. genuinely missing non-libc symbols.
    u=$(ldd -r "$R_SO" 2>&1 | grep -i 'undefined symbol' || true)
    if [ -n "$u" ]; then echo "  UNRESOLVED:"; echo "$u"; fail=1;
    else echo "  none — all imports resolve (libc/libgcc only)"; fi
    echo "    allocator imports: $(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
        | sed 's/@.*//' | grep -Ecx 'malloc|realloc|free') / 3 present"

    echo "--- differential tests ---"
    if timeout 600 cargo test --offline "${relargs[@]}" "${featargs[@]}" \
         -- --test-threads=1 2>&1 \
         | grep -E 'Running|test result|^error|FAILED|panicked' | sed 's/^/  /'; then :; else
      echo "  TESTS FAILED"; fail=1
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then echo "ALL PHASE D CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
