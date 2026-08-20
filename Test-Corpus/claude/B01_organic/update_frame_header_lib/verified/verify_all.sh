#!/usr/bin/env bash
# Phase D driver: enumerate every build-time configuration mechanically, then
# run cargo check / build / symbol-diff / cargo test for each of them.
#
# Usage: ./verify_all.sh [--exhaustive]
#   --exhaustive  also run the #[ignore]d full-2^32 sweeps (~10 min)
set -uo pipefail
cd "$(dirname "$0")"

EXHAUSTIVE=0
[[ "${1:-}" == "--exhaustive" ]] && EXHAUSTIVE=1

LOGDIR="${TMPDIR:-/tmp}"
C_SO="c_src/build/libtranslated_rust.so"
RUST_SO_NAME="libupdate_frame_header_lib.so"
FAILED=0

hdr() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()  { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad() { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
# 1. Enumerate every valid feature combination straight out of Cargo.toml.
# ---------------------------------------------------------------------------
hdr "Feature enumeration (from Cargo.toml)"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
echo "  declared non-default features: ${#FEATURES[@]} (${FEATURES[*]:-none})"

# Power set of FEATURES; with 0 features this yields exactly one combo: "".
COMBOS=("")
for f in "${FEATURES[@]}"; do
  new=()
  for c in "${COMBOS[@]}"; do
    new+=("$c")
    if [[ -z "$c" ]]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  COMBOS=("${new[@]}")
done
echo "  feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "    - '${c:-<none>}'"; done

# ---------------------------------------------------------------------------
# 2. Build the C shared object (single CMake configuration - no options exist).
# ---------------------------------------------------------------------------
hdr "Build C shared object"
if [[ ! -f "$C_SO" ]]; then
  (mkdir -p c_src/build && cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || bad "cmake build"
fi
[[ -f "$C_SO" ]] && ok "$C_SO" || bad "$C_SO missing"

# ---------------------------------------------------------------------------
# 3. For every combo x profile: check, build, symbol-diff, test.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    label="features='${combo:-<none>}' profile=$profile"
    relflag=""; [[ "$profile" == release ]] && relflag="--release"

    hdr "$label"

    cargo check --no-default-features --features "$combo" $relflag >"$LOGDIR/vc.$$" 2>&1 \
      && ok "cargo check" || { bad "cargo check"; tail -20 "$LOGDIR/vc.$$"; }

    cargo build --no-default-features --features "$combo" $relflag >"$LOGDIR/vb.$$" 2>&1 \
      && ok "cargo build" || { bad "cargo build"; tail -20 "$LOGDIR/vb.$$"; }

    RUST_SO="target/$profile/$RUST_SO_NAME"
    if [[ -f "$RUST_SO" ]]; then
      diff <(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort) \
           <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort) >"$LOGDIR/vd.$$" 2>&1 \
        && ok "symbol parity (nm -D): 0 missing" \
        || { bad "symbol parity"; cat "$LOGDIR/vd.$$"; }
      # No undefined non-libc symbols: dlopen would fail otherwise; check
      # explicitly too.
      undef=$(nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' \
              | grep -v '^_ITM_\|@GLIBC\|@GCC\|^_Unwind\|^__\|^abort$\|^bcmp$\|^calloc$\|^close$\|^free$\|^getcwd$\|^getenv$\|^malloc$\|^memcpy$\|^memmove$\|^memset$\|^read$\|^readlink$\|^realloc$\|^realpath$\|^strlen$\|^syscall$\|^write$\|^writev$\|^dl_iterate_phdr$\|^pthread_\|^statx$\|^gettid$\|^mmap64$\|^munmap$\|^open64$\|^lseek64$\|^fstat64$\|^stat64$\|^posix_memalign$' || true)
      if [[ -z "$undef" ]]; then ok "no undefined non-libc symbols"; else bad "undefined: $undef"; fi
    else
      bad "$RUST_SO missing"
    fi

    if [[ $EXHAUSTIVE -eq 1 ]]; then
      timeout 590 cargo test --no-default-features --features "$combo" $relflag \
        -- --include-ignored --test-threads=1 >"$LOGDIR/vt.$$" 2>&1
    else
      timeout 590 cargo test --no-default-features --features "$combo" $relflag >"$LOGDIR/vt.$$" 2>&1
    fi
    if [[ $? -eq 0 ]]; then
      ok "cargo test ($(grep -c '^test .* ok$' "$LOGDIR/vt.$$") tests ok)"
      grep -E '^test result:' "$LOGDIR/vt.$$" | sed 's/^/       /'
    else
      bad "cargo test"; tail -40 "$LOGDIR/vt.$$"
    fi
  done
done

rm -f "$LOGDIR/vc.$$" "$LOGDIR/vb.$$" "$LOGDIR/vd.$$" "$LOGDIR/vt.$$"
hdr "RESULT"
if [[ $FAILED -eq 0 ]]; then echo "  ALL CONFIGURATIONS VERIFIED"; else echo "  FAILURES PRESENT"; fi
exit $FAILED
