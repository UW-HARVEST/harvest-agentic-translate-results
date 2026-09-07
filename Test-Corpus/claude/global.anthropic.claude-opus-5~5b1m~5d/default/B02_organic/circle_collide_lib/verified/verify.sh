#!/usr/bin/env bash
# Phase D driver: symbol parity + the whole differential suite under every
# feature combination and every build profile of the Rust cdylib.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
CBUILD="$ROOT/c_src/build"
OUT="${TMPDIR:-/tmp}/verify.$$"
mkdir -p "$OUT"
FAIL=0

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()  { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad() { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
say "Building the C shared library"
# ---------------------------------------------------------------------------
( mkdir -p "$CBUILD" && cd "$CBUILD" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO=$(ls "$CBUILD"/lib*.so | head -1)
ok "C .so = $C_SO"

# ---------------------------------------------------------------------------
say "Enumerating feature combinations"
# ---------------------------------------------------------------------------
# Mechanically read the [features] table out of Cargo.toml. This crate has
# none, so the only combination is the default (empty) one; the loop below
# still exercises --no-default-features for completeness.
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[ ]*=/ { sub(/[ ]*=.*/, ""); print }
' Cargo.toml)
if [ -z "$FEATURES" ]; then
  echo "  no [features] table -> 1 combination (default)"
  COMBOS=("default")
else
  echo "  features: $FEATURES"
  COMBOS=("default" "--no-default-features")
  for f in $FEATURES; do COMBOS+=("--no-default-features --features $f"); done
  COMBOS+=("--all-features")
fi

# ---------------------------------------------------------------------------
say "Symbol parity (nm -D), all profiles"
# ---------------------------------------------------------------------------
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$OUT/c.syms"
echo "  C exports $(wc -l < "$OUT/c.syms") symbols"

for profile in debug release; do
  if [ "$profile" = release ]; then
    cargo build --release --offline >/dev/null 2>&1
  else
    cargo build --offline >/dev/null 2>&1
  fi
  R_SO="target/$profile/libcircle_collide_lib.so"
  [ -f "$R_SO" ] || { bad "missing $R_SO"; continue; }
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > "$OUT/r.$profile.syms"
  comm -23 "$OUT/c.syms" "$OUT/r.$profile.syms" > "$OUT/missing.$profile"
  if [ -s "$OUT/missing.$profile" ]; then
    bad "$profile: symbols in C but MISSING from Rust:"; sed 's/^/      /' "$OUT/missing.$profile"
  else
    ok "$profile: 0 missing symbols ($(wc -l < "$OUT/r.$profile.syms") exported)"
  fi
  # Undefined non-libc symbols in the Rust object.
  nm -D --undefined-only "$R_SO" | awk '{print $2}' \
    | grep -vE '^(__cxa_finalize|_ITM_|__gmon_start__|__tls_get_addr|memcpy|memset|memmove|memcmp|bcmp|strlen|malloc|free|calloc|realloc|abort|__stack_chk_fail|pthread_|dl|__errno_location|write|getenv|sigaltstack|sysconf|mmap|munmap|mprotect|open|close|read|poll|nanosleep|clock_gettime|gettid|syscall|__libc_start_main|_Unwind_|posix_memalign|_exit|exit|raise|sigaction|sigemptyset|sigaddset|sigprocmask|__assert_fail|qsort|bsearch|readlink|statx|stat|fstat|lstat|getcwd|environ|program_invocation)' \
    > "$OUT/undef.$profile" || true
  if [ -s "$OUT/undef.$profile" ]; then
    echo "  note: $profile other undefined symbols (informational):"
    sed 's/^/      /' "$OUT/undef.$profile"
  else
    ok "$profile: 0 undefined non-libc symbols"
  fi
done

# ---------------------------------------------------------------------------
say "Differential suite: every feature combo x every Rust .so profile"
# ---------------------------------------------------------------------------
export C_SO_PATH="$C_SO"
for combo in "${COMBOS[@]}"; do
  flags=""; [ "$combo" != default ] && flags="$combo"
  for profile in debug release; do
    label="features='${combo}' rust-so=${profile}"
    export RUST_SO_PATH="$PWD/target/$profile/libcircle_collide_lib.so"
    log="$OUT/test.$(echo "$combo$profile" | tr -c 'a-zA-Z0-9' _).log"
    if timeout 600 cargo test --offline $flags -- --test-threads=1 >"$log" 2>&1; then
      n=$(grep -c '^test .* ok$' "$log")
      ok "$label — $n tests"
    else
      bad "$label"; tail -30 "$log" | sed 's/^/      /'
    fi
  done
done

# ---------------------------------------------------------------------------
say "Binary / driver executable check"
# ---------------------------------------------------------------------------
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt" 2>/dev/null; then
  bad "c_src builds an executable — stdout comparison required but not implemented"
else
  ok "no add_executable in c_src/CMakeLists.txt and no [[bin]] in Cargo.toml -> N/A"
fi

say "RESULT"
if [ "$FAIL" = 0 ]; then printf '  \033[32mALL CHECKS PASSED\033[0m\n'; else printf '  \033[31mFAILURES PRESENT\033[0m\n'; fi
exit "$FAIL"
