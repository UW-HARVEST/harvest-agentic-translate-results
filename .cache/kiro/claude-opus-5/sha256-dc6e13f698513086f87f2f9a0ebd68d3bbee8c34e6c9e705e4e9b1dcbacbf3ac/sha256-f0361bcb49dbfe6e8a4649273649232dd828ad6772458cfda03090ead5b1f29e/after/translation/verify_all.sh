#!/usr/bin/env bash
# Full verification: build the C .so and the Rust .so, diff their exported
# symbols, and run the differential suite under every Cargo feature
# combination declared in Cargo.toml.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"
R_SO="$HERE/target/release/libdriver.so"

echo "== building C shared library =="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )
test -f "$C_SO"

echo "== building Rust shared library =="
( cd "$HERE" && timeout 600 cargo build --release )

echo
echo "== Phase D: symbol diff (must be empty) =="
diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
     <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort) \
  && echo "symbol diff: EMPTY (parity)"

echo
echo "== undefined (imported) symbols in the Rust .so =="
nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sort -u

echo
echo "== enumerating feature combinations from Cargo.toml =="
# Read the [features] table, if any. Powerset of the declared features, always
# including the default build and the --no-default-features build.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$HERE/Cargo.toml"
)
echo "declared non-default features: ${FEATURES[*]:-<none>}"

COMBOS=("default" "no-default-features")
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("features:$combo")
  done
fi

echo
for c in "${COMBOS[@]}"; do
  case "$c" in
    default)              args=() ;;
    no-default-features)  args=(--no-default-features) ;;
    features:*)           args=(--no-default-features --features "${c#features:}") ;;
  esac
  echo "== combination: $c =="
  ( cd "$HERE" \
    && timeout 600 cargo build --release "${args[@]}" >/dev/null \
    && timeout 600 cargo test  --release "${args[@]}" -- --nocapture 2>&1 \
       | sed -n '/^  \[ok\]/p;/rows passed/p;/^test result/p' )
  echo
done

echo "ALL COMBINATIONS PASSED"
