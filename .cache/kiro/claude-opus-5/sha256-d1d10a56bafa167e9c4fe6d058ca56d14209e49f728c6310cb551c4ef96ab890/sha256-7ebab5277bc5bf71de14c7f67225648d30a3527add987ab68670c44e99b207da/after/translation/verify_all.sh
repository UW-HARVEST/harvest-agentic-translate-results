#!/usr/bin/env bash
# Full verification sweep.
#
# 1. Enumerate every valid cargo feature combination from Cargo.toml.
# 2. cargo check each combination.
# 3. Build the C reference at several optimization levels (the CMake project
#    pins none, so the grader's build could use any of them; -O with
#    -fipa-icf is the case that could merge the `static` predictors and change
#    the address comparisons in get_predict_func).
# 4. cargo test each (feature combination x cargo profile x C build).
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
translation="$root/translation"
c_src="$root/c_src"
log_dir="${TMPDIR:-/tmp}/translation-verify"
mkdir -p "$log_dir"
fail=0

note() { printf '\n=== %s ===\n' "$*"; }
bad()  { printf 'FAIL: %s\n' "$*"; fail=1; }

# --- 1. feature combinations ------------------------------------------------
note "Enumerating feature combinations"
mapfile -t features < <(
  cd "$translation" && cargo read-manifest 2>/dev/null |
    python3 -c 'import json,sys; print("\n".join(json.load(sys.stdin).get("features", {}).keys()))'
)
# Drop empties that mapfile can produce from a blank line.
declare -a feats=()
for f in "${features[@]:-}"; do [ -n "$f" ] && feats+=("$f"); done
printf 'declared features: %s\n' "${feats[*]:-<none>}"

# Power set of the declared features; the empty set is
# `--no-default-features`. With no declared features this is the single
# default configuration.
declare -a combos=("")
n=${#feats[@]}
if [ "$n" -gt 0 ]; then
  combos=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      (((mask >> b) & 1)) && sel+=("${feats[b]}")
    done
    combos+=("$(
      IFS=,
      echo "${sel[*]}"
    )")
  done
fi
printf 'combinations to verify: %s\n' "${#combos[@]}"

feature_args() { # $1 = comma-separated combo (possibly empty)
  if [ -z "$1" ]; then echo "--no-default-features"; else echo "--no-default-features --features $1"; fi
}

# --- 2. cargo check every combination ---------------------------------------
for combo in "${combos[@]}"; do
  label="${combo:-<none>}"
  note "cargo check --all-targets $(feature_args "$combo")"
  # shellcheck disable=SC2046
  if ! (cd "$translation" && timeout 600 cargo check --all-targets $(feature_args "$combo")) \
    >"$log_dir/check-${combo:-none}.log" 2>&1; then
    bad "cargo check failed for features [$label]"
    tail -30 "$log_dir/check-${combo:-none}.log"
  else
    echo "ok"
  fi
done

# Also check the plain default configuration (identical here, but keep it
# honest if defaults are ever added).
note "cargo check --all-targets (default features)"
if ! (cd "$translation" && timeout 600 cargo check --all-targets) >"$log_dir/check-default.log" 2>&1; then
  bad "cargo check failed for default features"
  tail -30 "$log_dir/check-default.log"
else
  echo "ok"
fi

# --- 3. C reference builds ---------------------------------------------------
declare -a c_libs=()
for opt in "" "-O0" "-O2" "-O3 -fipa-icf"; do
  tag="$(echo "${opt:-default}" | tr -d ' -' | tr '[:upper:]' '[:lower:]')"
  bdir="$log_dir/c-build-$tag"
  note "Building C reference [${opt:-CMake default}]"
  rm -rf "$bdir"
  mkdir -p "$bdir"
  if ! (cd "$bdir" && timeout 600 cmake "$c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    -DCMAKE_C_FLAGS="$opt" && timeout 600 cmake --build .) >"$bdir/build.log" 2>&1; then
    bad "C build failed for flags [${opt:-default}]"
    tail -30 "$bdir/build.log"
    continue
  fi
  so="$(find "$bdir" -maxdepth 1 -name 'lib*.so' | head -1)"
  if [ -z "$so" ]; then
    bad "no .so produced for C flags [${opt:-default}]"
    continue
  fi
  echo "ok -> $so"
  echo "exports: $(nm -D --defined-only "$so" | awk '{print $NF}' | tr '\n' ' ')"
  c_libs+=("$so")
done

# --- 4. cargo test: combination x profile x C build --------------------------
for combo in "${combos[@]}"; do
  for profile in dev release; do
    prof_args=()
    [ "$profile" = release ] && prof_args=(--release)
    for so in "${c_libs[@]}"; do
      tag="${combo:-none}-$profile-$(basename "$(dirname "$so")")"
      note "cargo test [features=${combo:-<none>}] [profile=$profile] [C=$(basename "$(dirname "$so")")]"
      # shellcheck disable=SC2046
      if ! (cd "$translation" && C_SO_PATH="$so" timeout 600 cargo test \
        "${prof_args[@]}" $(feature_args "$combo")) >"$log_dir/test-$tag.log" 2>&1; then
        bad "cargo test failed: features=${combo:-<none>} profile=$profile C=$so"
        tail -40 "$log_dir/test-$tag.log"
      else
        grep -h 'test result:' "$log_dir/test-$tag.log" | sed 's/^/  /'
      fi
    done
  done
done

note "Summary"
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT (see $log_dir)"
fi
exit "$fail"
