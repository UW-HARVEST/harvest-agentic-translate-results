#!/usr/bin/env bash
# Phase A / Phase D: enumerate every build configuration of this crate and run
# `cargo check` + the whole differential test suite for each of them.
#
# `Cargo.toml` declares no `[features]` and `c_src/CMakeLists.txt` has no
# configuration options, so the complete set of feature combinations is
#   1. (default, i.e. no features)
#   2. --no-default-features   (identical to 1, but verified explicitly)
# The list is derived from Cargo.toml at run time, so it stays correct if
# features are ever added.

set -u
cd "$(dirname "$0")" || exit 1

CARGO_FLAGS=${CARGO_FLAGS:---offline}

# --- enumerate the features declared in Cargo.toml --------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /=/     { split($0, a, "="); gsub(/[ \t]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS+=("")            # default (= empty) feature set
  COMBOS+=("__NONE__")    # --no-default-features
else
  # power set of all declared features, with and without default features
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="$combo,${FEATURES[$i]}"; fi
    done
    COMBOS+=("${combo#,}")
  done
fi

echo "feature combinations to verify: ${#COMBOS[@]}"

fail=0
run() { # run <label> <cargo subcommand> <extra args...>
  local label="$1"; shift
  echo "=== $label: cargo $* ==="
  if ! cargo "$@"; then
    echo "!!! FAILED: $label ($*)"
    fail=1
  fi
}

# The C artifacts are (re)built by the test harness itself; do it once up front
# so a build error is reported before the first cargo run.
mkdir -p c_src/build
gcc -shared -fPIC -O2 -Ic_src/include -o c_src/build/libtextanalyzer_c.so \
    c_src/src/tokenizer.c c_src/src/analyzer.c c_src/src/main.c || fail=1
gcc -O2 -Ic_src/include -o c_src/build/driver \
    c_src/src/tokenizer.c c_src/src/analyzer.c c_src/src/main.c || fail=1

for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "__NONE__" ]; then
    label="--no-default-features"
    args=(--no-default-features)
  elif [ -z "$combo" ]; then
    label="(default)"
    args=()
  else
    label="--features $combo"
    args=(--no-default-features --features "$combo")
  fi

  run "$label" check $CARGO_FLAGS "${args[@]}" --all-targets
  run "$label" build $CARGO_FLAGS "${args[@]}"
  run "$label" test  $CARGO_FLAGS "${args[@]}"
  # the release profile differs (panic = "abort", optimisations): verify it too
  run "$label release" test --release $CARGO_FLAGS "${args[@]}"
done

if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit "$fail"
