#!/usr/bin/env bash
# Full verification matrix: enumerate every valid cargo feature combination,
# `cargo check` each, build the C .so, diff the exported symbols, then run the
# whole differential suite (Phase B + Phase C) for every combination and for
# both the debug and the release build of the Rust cdylib.
set -uo pipefail
cd "$(dirname "$0")"
fail=0
step() { printf '\n=== %s ===\n' "$*"; }

# --- enumerate features from Cargo.toml -----------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[ \t]*=/ {
      split($0, a, "="); gsub(/[ \t]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
echo "declared non-default features: ${#FEATURES[@]} (${FEATURES[*]-none})"

# Power set of the declared features; with none declared this is exactly one
# combination: the empty one.
COMBOS=("")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  total=$((1 << n))
  COMBOS=()
  for ((m = 0; m < total; m++)); do
    sel=""
    for ((b = 0; b < n; b++)); do
      if (((m >> b) & 1)); then sel="${sel:+$sel,}${FEATURES[$b]}"; fi
    done
    COMBOS+=("$sel")
  done
fi
echo "feature combinations to verify: ${#COMBOS[@]}"

# --- cargo check for every combination ------------------------------------
for combo in "${COMBOS[@]}"; do
  step "cargo check --no-default-features --features '$combo'"
  timeout 600 cargo check --offline --no-default-features --features "$combo" --all-targets \
    || { echo "CHECK FAILED for '$combo'"; fail=1; }
done
step "cargo check (default features)"
timeout 600 cargo check --offline --all-targets || fail=1
step "cargo check --all-features"
timeout 600 cargo check --offline --all-features --all-targets || fail=1

# --- build the C reference shared object ----------------------------------
step "build C .so"
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null) || { echo "C BUILD FAILED"; exit 1; }
C_SO=c_src/build/libtranslated_rust.so
ls -l "$C_SO"

# --- run the suite for every combination, debug and release ---------------
for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    step "TEST features='$combo' rust-profile=$profile"
    if [ "$profile" = release ]; then
      timeout 600 cargo build --offline --release --no-default-features --features "$combo" \
        || { fail=1; continue; }
      SO=target/release/libcapsule_lib.so
    else
      timeout 600 cargo build --offline --no-default-features --features "$combo" \
        || { fail=1; continue; }
      SO=target/debug/libcapsule_lib.so
    fi

    # Phase D symbol diff for this configuration.
    nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort > /tmp/.c_syms.$$ 2>/dev/null \
      || nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort > "${TMPDIR:-.}/c_syms.$$"
    csyms="${TMPDIR:-/tmp}/c_syms.$$"
    nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort > "$csyms"
    rsyms="${TMPDIR:-/tmp}/r_syms.$$"
    nm -D --defined-only "$SO" | awk '$2=="T"{print $3}' | sort > "$rsyms"
    missing=$(comm -23 "$csyms" "$rsyms")
    if [ -n "$missing" ]; then
      echo "SYMBOL PARITY FAILED ($profile, '$combo'): missing $missing"
      fail=1
    else
      echo "symbol parity OK: $(wc -l < "$csyms") C symbols, all present in $SO"
    fi
    rm -f "$csyms" "$rsyms" /tmp/.c_syms.$$

    HARVEST_RUST_SO="$PWD/$SO" timeout 600 cargo test --offline \
      --no-default-features --features "$combo" -- --test-threads=4 \
      || { echo "TESTS FAILED ($profile, '$combo')"; fail=1; }
  done
done

step "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
