#!/usr/bin/env bash
# Build C + Rust, then run every differential test under every feature combo.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"

echo "=== building C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

# Feature combinations declared in Cargo.toml ([features] section).
mapfile -t FEATS < <(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]); if(a[1]!="default") print a[1]}' "$ROOT/translation/Cargo.toml")

run_combo() {
  local desc="$1"; shift
  echo "=== $desc ==="
  ( cd "$ROOT/translation" \
    && timeout 600 cargo build --release "$@" >/dev/null 2>&1 \
    && nm -D --defined-only target/release/libdriver.so | awk '$2~/^[TBD]$/{print $3}' | sort > /tmp/rs_syms.txt \
    && nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '$2~/^[TBD]$/{print $3}' | sort > /tmp/c_syms.txt \
    && { d=$(comm -23 /tmp/c_syms.txt /tmp/rs_syms.txt); [ -z "$d" ] || { echo "SYMBOL DIFF (missing in Rust): $d"; exit 1; }; } \
    && timeout 600 cargo test --release "$@" -- --test-threads=1 ) \
    || { echo "FAILED: $desc"; return 1; }
}

rc=0
run_combo "default features" || rc=1
if [ "${#FEATS[@]}" -gt 0 ]; then
  run_combo "no-default-features" --no-default-features || rc=1
  for f in "${FEATS[@]}"; do
    run_combo "feature: $f" --no-default-features --features "$f" || rc=1
  done
  all=$(IFS=,; echo "${FEATS[*]}")
  run_combo "all features: $all" --no-default-features --features "$all" || rc=1
  run_combo "--all-features" --all-features || rc=1
else
  echo "=== no [features] section in Cargo.toml: exactly one configuration exists ==="
fi
exit $rc
