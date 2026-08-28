#!/usr/bin/env bash
# Verifies the Rust translation against the C reference for every build-time
# configuration:
#
#   * every feature combination declared in translation/Cargo.toml
#   * both the debug and release profiles
#
# The crate currently declares no [features], so the feature loop degenerates to
# the single default configuration; the loop is written generically so that
# adding a feature is picked up automatically.
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$root/translation"
log_dir="${TMPDIR:-/tmp}/c2rust-verify-logs"
mkdir -p "$log_dir"
rc=0

run() { # run <logname> <cmd...>
  local name="$1"; shift
  local log="$log_dir/$name.log"
  if timeout 600 "$@" >"$log" 2>&1; then
    echo "  PASS  $name"
  else
    echo "  FAIL  $name  (see $log)"
    tail -n 25 "$log" | sed 's/^/        /'
    rc=1
  fi
}

echo "== building the C reference library =="
mkdir -p "$root/c_src/build"
run cmake-configure cmake -S "$root/c_src" -B "$root/c_src/build" \
  -DCMAKE_POSITION_INDEPENDENT_CODE=ON
run cmake-build cmake --build "$root/c_src/build"

# ---------------------------------------------------------------------------
# Enumerate every valid feature combination from Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t features < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1])
      if (a[1] != "default") print a[1]
    }
  ' "$crate/Cargo.toml"
)

combos=("")
for f in "${features[@]:-}"; do
  [ -z "$f" ] && continue
  new=()
  for c in "${combos[@]}"; do
    new+=("$c")
    if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  combos=("${new[@]}")
done

echo "== feature combinations: ${#combos[@]} =="
for c in "${combos[@]}"; do
  echo "   - '${c:-<none>}'"
done

cd "$crate" || exit 1

for c in "${combos[@]}"; do
  label="${c:-none}"
  echo
  echo "== combination: ${label} =="
  if [ -z "$c" ]; then
    featargs=(--no-default-features)
  else
    featargs=(--no-default-features --features "$c")
  fi

  run "check-$label"          cargo check "${featargs[@]}" --all-targets
  run "build-$label"          cargo build "${featargs[@]}" --release
  run "test-debug-$label"     cargo test "${featargs[@]}" -- --test-threads=1
  run "test-release-$label"   cargo test "${featargs[@]}" --release -- --test-threads=1

  # Symbol parity for this combination.
  c_so="$root/c_src/build/libdriver.so"
  rs_so="$crate/target/release/libdriver.so"
  syms_c="$log_dir/syms-c.txt"
  syms_rs="$log_dir/syms-rs-$label.txt"
  nm -D --defined-only "$c_so"  | awk '$2 ~ /^[TtDBRWVGSi]$/ {print $3}' | sort -u > "$syms_c"
  nm -D --defined-only "$rs_so" | awk '$2 ~ /^[TtDBRWVGSi]$/ {print $3}' | sort -u > "$syms_rs"
  if missing="$(comm -23 "$syms_c" "$syms_rs")" && [ -z "$missing" ]; then
    echo "  PASS  symbols-$label ($(wc -l < "$syms_c") exported by C, all present in Rust)"
  else
    echo "  FAIL  symbols-$label: Rust is missing:"
    echo "$missing" | sed 's/^/        /'
    rc=1
  fi
done

echo
if [ "$rc" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASS"
else
  echo "FAILURES PRESENT"
fi
exit "$rc"
