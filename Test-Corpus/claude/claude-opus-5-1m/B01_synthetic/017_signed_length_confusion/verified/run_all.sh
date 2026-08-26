#!/usr/bin/env bash
# Full verification matrix: build the C artifacts, build the Rust artifacts for
# every build configuration, compare exported symbols, then run both
# differential suites.
#
# Usage: ./run_all.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$PWD
FAIL=0
TMP=${TMPDIR:-/tmp}

say() { printf '\n=== %s ===\n' "$*"; }
ok()  { printf '  [ok]   %s\n' "$*"; }
bad() { printf '  [FAIL] %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
say "Feature combinations"
# ---------------------------------------------------------------------------
# Cargo.toml declares no [features], so there is exactly one combination.
# Enumerate mechanically anyway, so a future feature cannot be missed.
FEATURES=$(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip()
            if n != 'default':
                names.append(n)
print(' '.join(names))
PY
)
if [ -z "$FEATURES" ]; then
  echo "  no [features] in Cargo.toml -> 1 configuration (default == --no-default-features)"
  COMBOS=("" "--no-default-features")
else
  echo "  features: $FEATURES"
  COMBOS=("" "--no-default-features")
  # Power set of the declared features.
  n=$(echo "$FEATURES" | wc -w)
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""; i=0
    for f in $FEATURES; do
      if (( (mask>>i) & 1 )); then combo="$combo,$f"; fi
      i=$((i+1))
    done
    COMBOS+=("--no-default-features --features ${combo#,}")
  done
fi

# ---------------------------------------------------------------------------
say "Building the C artifacts (c_src is never modified)"
# ---------------------------------------------------------------------------
mkdir -p c_src/build
( cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null 2>&1 \
  && cmake --build . >/dev/null 2>&1 ) \
  && ok "cmake executable: c_src/build/driver" || bad "cmake build failed"

mkdir -p target/c_build
gcc -shared -fPIC -o target/c_build/libdriver_c.so c_src/src/main.c \
  && ok "shared object: target/c_build/libdriver_c.so" || bad "gcc -shared failed"

# ---------------------------------------------------------------------------
say "cargo check for every feature combination"
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="cargo check ${combo:-<default>}"
  if timeout 300 cargo check --offline --all-targets $combo >"$TMP/check.log" 2>&1; then
    ok "$label"
  else
    bad "$label"; tail -20 "$TMP/check.log"
  fi
done

# ---------------------------------------------------------------------------
say "Symbol parity: C .so vs Rust .so"
# ---------------------------------------------------------------------------
timeout 400 cargo build --offline --release --lib >/dev/null 2>&1
filter() { grep -v -E ' (_init|_fini|_edata|_end|_IO_stdin_used)$' | grep -v -E ' __' ; }
nm -D --defined-only target/c_build/libdriver_c.so | awk '$2=="T"{print $3}' | filter | sort > "$TMP/c.syms"
nm -D --defined-only target/release/libdriver.so   | awk '$2=="T"{print $3}' | filter | sort > "$TMP/r.syms"
echo "  C   .so exports: $(tr '\n' ' ' < "$TMP/c.syms")"
echo "  Rust .so exports: $(tr '\n' ' ' < "$TMP/r.syms")"
# `main` is the process entry point, not a library API: Rust's bin target
# generates its own `main`, so re-exporting it from the rlib is a hard link
# error. Its equivalence is covered executable-to-executable instead.
MISSING=$(comm -23 "$TMP/c.syms" "$TMP/r.syms" | grep -v '^main$')
if [ -z "$MISSING" ]; then
  ok "every C .so function symbol is exported by the Rust .so (except entry point 'main')"
else
  bad "missing from the Rust .so: $(echo "$MISSING" | tr '\n' ' ')"
fi
for exe in c_src/build/driver target/release/driver; do
  if nm --defined-only "$exe" 2>/dev/null | awk '$2=="T"{print $3}' | grep -qx main; then
    ok "$exe defines 'main'"
  else
    bad "$exe does not define 'main'"
  fi
done
# Undefined entries are fine when they are resolved by the dynamic loader:
# glibc (`@GLIBC_*`), the libgcc unwinder (`@GCC_*`), or weak markers (`w`,
# e.g. _ITM_*/__gmon_start__/statx) which stay unresolved by design.
UNDEF=$(nm -D -u target/release/libdriver.so \
        | grep -v -E '^\s+w ' \
        | grep -v -E '@(GLIBC|GCC)_' )
if [ -n "$UNDEF" ]; then
  bad "Rust .so has undefined non-libc symbols:"; echo "$UNDEF"
else
  ok "Rust .so has no undefined non-libc symbols (all resolve to glibc/libgcc)"
fi

# ---------------------------------------------------------------------------
say "Differential test suites"
# ---------------------------------------------------------------------------
# Both Rust profiles are compared against the same C build: `release` carries
# `panic = "abort"`, `dev` carries debug assertions and bounds checks.
for profile in dev release; do
  for combo in "${COMBOS[@]}"; do
    flag=""; [ "$profile" = release ] && flag="--release"
    label="tests profile=$profile ${combo:-<default>}"
    # Build lib + examples first: `cargo test --test X` alone can leave a stale
    # cdylib/example behind (the suites hard-fail on stale artifacts).
    timeout 400 cargo build --offline $flag $combo --lib --examples >/dev/null 2>&1
    if timeout 600 cargo test --offline $flag $combo >"$TMP/test.log" 2>&1; then
      ok "$label -- $(grep -h 'test result:' "$TMP/test.log" | tr '\n' ' ')"
    else
      bad "$label"
      grep -E 'diverged|panicked|test result:|^test .* FAILED' "$TMP/test.log" | head -40
    fi
  done
done

say "Summary"
if [ "$FAIL" -eq 0 ]; then
  echo "  ALL CHECKS PASSED"
else
  echo "  FAILURES PRESENT (see above)"
fi
exit "$FAIL"
