#!/usr/bin/env bash
# Phase D driver: enumerate every build-time configuration and run the whole
# differential suite in each of them.
#
#   * Cargo feature combinations are read out of Cargo.toml. This crate has no
#     [features] section, so the matrix is exactly {default, --no-default-features}.
#   * Both cargo profiles are exercised, because the release profile sets
#     `panic = "abort"`, which changes how the cdylib is built.
#   * c_src/CMakeLists.txt has no options, so the C side has a single config.
set -uo pipefail
cd "$(dirname "$0")"

fail=0
log() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------- feature matrix
FEATURES=$(python3 - <<'EOF'
import itertools, re, sys, pathlib
txt = pathlib.Path("Cargo.toml").read_text()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip()
            if n and n != 'default':
                names.append(n)
combos = []
for k in range(len(names) + 1):
    for c in itertools.combinations(names, k):
        combos.append(",".join(c))
print("\n".join(combos if combos else [""]))
EOF
)
log "feature combinations (empty line == no features)"
echo "$FEATURES" | sed 's/^$/<none>/'

# ------------------------------------------------------------------- C library
log "building the C shared library"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
ls -l c_src/build/libtranslated_rust.so

# --------------------------------------------------------------- cargo check
while IFS= read -r combo; do
  log "cargo check --no-default-features --features '${combo}'"
  if ! timeout 300 cargo check --no-default-features --features "$combo" --all-targets 2>&1 | tail -5; then
    echo "CHECK FAILED for '${combo}'"; fail=1
  fi
done <<< "$FEATURES"

# --------------------------------------------------------------- cargo test
for profile in dev release; do
  flag=""; [ "$profile" = release ] && flag="--release"
  while IFS= read -r combo; do
    log "cargo build ${flag} --no-default-features --features '${combo}' (cdylib under test)"
    if ! timeout 300 cargo build $flag --no-default-features --features "$combo" 2>&1 | tail -3; then
      echo "BUILD FAILED for '${combo}' (${profile})"; fail=1
    fi
    log "cargo test ${flag} --no-default-features --features '${combo}'"
    if ! timeout 550 cargo test $flag --no-default-features --features "$combo" \
           -- --test-threads=1 2>&1 | grep -E "^(running|test result|error|test .* FAILED)"; then
      echo "TEST RUN PRODUCED NO SUMMARY for '${combo}' (${profile})"; fail=1
    fi
    # explicit pass/fail detection
    if ! timeout 550 cargo test $flag --no-default-features --features "$combo" \
           -- --test-threads=1 >"$TMPDIR/.rt.$$" 2>&1; then
      echo "TESTS FAILED for '${combo}' (${profile})"; tail -30 "$TMPDIR/.rt.$$"; fail=1
    fi
    rm -f "$TMPDIR/.rt.$$"
  done <<< "$FEATURES"
done

# ----------------------------------------------------------- symbol diff (both)
for profile in debug release; do
  so="target/${profile}/libarr_del_lib.so"
  [ -f "$so" ] || continue
  log "nm -D symbol diff (C vs ${so})"
  nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort > "$TMPDIR/.c.$$"
  nm -D --defined-only "$so" | awk '{print $3}' | sort > "$TMPDIR/.r.$$"
  missing=$(comm -23 "$TMPDIR/.c.$$" "$TMPDIR/.r.$$")
  if [ -n "$missing" ]; then echo "MISSING FROM RUST:"; echo "$missing"; fail=1
  else echo "OK: all $(wc -l < "$TMPDIR/.c.$$") C symbols are exported by the Rust .so"; fi
  rm -f "$TMPDIR/.c.$$" "$TMPDIR/.r.$$"
done

log "RESULT"
[ "$fail" = 0 ] && echo "ALL CONFIGURATIONS PASSED" || echo "FAILURES PRESENT"
exit "$fail"
