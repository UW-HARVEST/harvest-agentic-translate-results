#!/usr/bin/env bash
# Enumerate every valid feature combination declared in translation/Cargo.toml
# and run cargo check + cargo test (debug and release) for each.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1

# Extract feature names from the [features] section, ignoring "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f=1; next }
    /^\[/           { in_f=0 }
    in_f && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      line=$0; sub(/[[:space:]]*=.*/, "", line); gsub(/[[:space:]]/, "", line);
      if (line != "default") print line
    }
  ' Cargo.toml
)

echo "Declared features: ${#FEATURES[@]} ${FEATURES[*]-<none>}"

# Build the list of combinations (powerset of declared features).
COMBOS=("")
for f in "${FEATURES[@]}"; do
  for existing in "${COMBOS[@]}"; do
    if [ -z "$existing" ]; then COMBOS+=("$f"); else COMBOS+=("$existing,$f"); fi
  done
done

echo "Combinations to verify: ${#COMBOS[@]}"

fail=0
run() {
  local desc="$1"; shift
  echo "--- $desc"
  if ! timeout 600 "$@" > /tmp/featcheck.log 2>&1; then
    echo "FAILED: $desc"
    tail -30 /tmp/featcheck.log
    fail=1
  fi
}

for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    label="<no features>"
    args=(--no-default-features)
  else
    label="$combo"
    args=(--no-default-features --features "$combo")
  fi
  run "check   [$label]" cargo check "${args[@]}"
  # cargo test does not emit the cdylib for a crate-type=["cdylib"] package,
  # so build it explicitly for both profiles before testing.
  run "build   [$label] debug" cargo build "${args[@]}"
  run "build   [$label] release" cargo build --release "${args[@]}"
  run "test    [$label] debug" cargo test "${args[@]}"
  run "test    [$label] release" cargo test --release "${args[@]}"

  # Symbol parity: every symbol the C .so exports must also exist in the
  # Rust .so, under both profiles.
  c_so=$(ls ../c_src/build/lib*.so 2>/dev/null | head -1)
  if [ -n "$c_so" ]; then
    nm -D --defined-only "$c_so" | awk '{print $3}' | sort -u > /tmp/syms_c.txt
    for prof in debug release; do
      rust_so="target/$prof/libmd5_digest_lib.so"
      [ -f "$rust_so" ] || { echo "FAILED: missing $rust_so"; fail=1; continue; }
      nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u > /tmp/syms_r.txt
      missing=$(comm -23 /tmp/syms_c.txt /tmp/syms_r.txt)
      if [ -n "$missing" ]; then
        echo "FAILED: [$label/$prof] symbols missing from Rust .so: $missing"
        fail=1
      else
        echo "--- symbols [$label/$prof] OK"
      fi
    done
  else
    echo "FAILED: C .so not built; run cmake first"
    fail=1
  fi
done

# Also exercise the default feature set explicitly.
run "check   [default]" cargo check
run "test    [default] debug" cargo test
run "test    [default] release" cargo test --release

if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS OK"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$fail"
