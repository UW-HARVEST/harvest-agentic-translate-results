#!/usr/bin/env bash
# Enumerate every valid feature combination declared in translation/Cargo.toml
# and run `cargo check` + `cargo test` for each.
#
# Usage: ./verify_all_features.sh [check|test|all]      (default: all)
set -uo pipefail

cd "$(dirname "$0")/translation" || exit 1
MODE="${1:-all}"
LOGDIR=/tmp/xlat-verify
mkdir -p "$LOGDIR"

# --- 1. Extract feature names from the [features] table -----------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/       { inblock = 1; next }
    /^\[/                 { inblock = 0 }
    inblock && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
        sub(/[[:space:]]*=.*/, "")
        gsub(/[[:space:]]/, "")
        if ($0 != "default") print
    }
  ' Cargo.toml
)

echo "features declared: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- 2. Build the list of combinations (powerset), plus the default build -----
COMBOS=("")                       # "" == --no-default-features with no features
n=${#FEATURES[@]}
if (( n > 0 )); then
  if (( n > 16 )); then
    echo "refusing to enumerate 2^$n combinations" >&2; exit 1
  fi
  for (( mask = 1; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "combinations to verify: ${#COMBOS[@]} (plus the default feature set)"

# --- 3. Make sure the C reference exists -------------------------------------
if ! compgen -G "../c_src/build/lib*.so" > /dev/null; then
  echo "building the C reference library"
  ( cd ../c_src && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      && cmake --build . ) > "$LOGDIR/cmake.log" 2>&1 \
    || { echo "FAIL: C build (see $LOGDIR/cmake.log)"; exit 1; }
fi

fail=0

run() {                            # run <label> <logname> <cargo args...>
  local label="$1" log="$LOGDIR/$2"; shift 2
  if timeout 600 cargo "$@" > "$log" 2>&1; then
    echo "  PASS  $label"
  else
    echo "  FAIL  $label   (log: $log)"
    tail -n 25 "$log" | sed 's/^/        /'
    fail=1
  fi
}

# --- 4. cargo check for every combination ------------------------------------
if [[ "$MODE" == check || "$MODE" == all ]]; then
  echo
  echo "== cargo check =="
  run "default features" "check-default.log" check --all-targets
  for combo in "${COMBOS[@]}"; do
    if [[ -z "$combo" ]]; then
      run "no-default-features" "check-none.log" \
          check --all-targets --no-default-features
    else
      run "features=$combo" "check-${combo//,/+}.log" \
          check --all-targets --no-default-features --features "$combo"
    fi
  done
fi

# --- 5. cargo test for every combination, debug and release ------------------
if [[ "$MODE" == test || "$MODE" == all ]]; then
  for profile in debug release; do
    echo
    echo "== cargo test ($profile) =="
    relflag=(); [[ $profile == release ]] && relflag=(--release)
    run "default features" "test-$profile-default.log" \
        test "${relflag[@]}"
    for combo in "${COMBOS[@]}"; do
      if [[ -z "$combo" ]]; then
        run "no-default-features" "test-$profile-none.log" \
            test "${relflag[@]}" --no-default-features
      else
        run "features=$combo" "test-$profile-${combo//,/+}.log" \
            test "${relflag[@]}" --no-default-features --features "$combo"
      fi
    done
  done
fi

# --- 6. Independent cross-check with a plain C driver ------------------------
# `betagamma` folds pointer comparisons into its result, so its value depends on
# the process heap history. This driver loads exactly one .so and drives it
# through an identical call sequence, so the two runs differ only in which
# implementation is under the hood -- no Rust test harness in the picture.
if [[ "$MODE" == test || "$MODE" == all ]] && command -v cc > /dev/null; then
  echo
  echo "== independent C driver cross-check =="
  cat > "$LOGDIR/driver.c" <<'EOF'
#include <dlfcn.h>
#include <stdio.h>
#include <stdint.h>

int main(int argc, char **argv) {
    if (argc < 2) { fprintf(stderr, "usage: driver <so>\n"); return 2; }
    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 2; }

    int (*betagamma)(int, int, int, int) = dlsym(h, "betagamma");
    int (*compute_hash)(void *, void *)  = dlsym(h, "compute_hash");
    void *(*allocate_block)(size_t, int) = dlsym(h, "allocate_block");
    void (*free_block)(void *)           = dlsym(h, "free_block");
    if (!betagamma || !compute_hash || !allocate_block || !free_block) {
        fprintf(stderr, "missing symbol\n"); return 2;
    }

    /* Deterministic sweep, identical for every library under test. */
    uint64_t s = 0x9e3779b97f4a7c15ULL;
    for (int k = 0; k < 3000; k++) {
        s ^= s >> 12; s ^= s << 25; s ^= s >> 27;
        uint64_t r = s * 0x2545F4914F6CDD1DULL;
        int a = (int)(uint32_t)(r >> 32);
        s ^= s >> 12; s ^= s << 25; s ^= s >> 27;
        r = s * 0x2545F4914F6CDD1DULL;
        int b = (int)(uint32_t)(r >> 32);
        printf("%d\n", betagamma(a, b, a ^ b, a + b));
        printf("%d\n", betagamma(k % 21 - 10, k, -k, k / 3));
    }
    /* Also drive the lower-level API so its allocation pattern is compared. */
    for (size_t n = 0; n < 40; n++) {
        void *m1 = allocate_block(n, (int)n - 20);
        void *m2 = allocate_block(n, (int)n + 20);
        printf("%d\n", compute_hash(m1, m2));
        free_block(m1);
        free_block(m2);
    }
    return 0;
}
EOF
  if cc "$LOGDIR/driver.c" -o "$LOGDIR/driver" -ldl > "$LOGDIR/driver-build.log" 2>&1; then
    c_so=$(ls ../c_src/build/lib*.so | head -1)
    for rs_so in target/release/libbetagamma_lib.so target/debug/libbetagamma_lib.so \
                 target/release/it-cdylib/release/libbetagamma_lib.so \
                 target/debug/it-cdylib/debug/libbetagamma_lib.so; do
      [[ -f "$rs_so" ]] || continue
      "$LOGDIR/driver" "$c_so"   > "$LOGDIR/driver-c.out"    2>&1
      "$LOGDIR/driver" "$rs_so"  > "$LOGDIR/driver-rust.out" 2>&1
      if diff -q "$LOGDIR/driver-c.out" "$LOGDIR/driver-rust.out" > /dev/null; then
        echo "  PASS  $(wc -l < "$LOGDIR/driver-c.out") values identical  ($rs_so)"
      else
        echo "  FAIL  $rs_so differs from $c_so"
        diff "$LOGDIR/driver-c.out" "$LOGDIR/driver-rust.out" | head -20 | sed 's/^/        /'
        fail=1
      fi
    done
  else
    echo "  SKIP  could not build the C driver (see $LOGDIR/driver-build.log)"
  fi
fi

echo
if (( fail )); then
  echo "RESULT: FAILURES (logs in $LOGDIR)"
else
  echo "RESULT: all configurations pass"
fi
exit $fail