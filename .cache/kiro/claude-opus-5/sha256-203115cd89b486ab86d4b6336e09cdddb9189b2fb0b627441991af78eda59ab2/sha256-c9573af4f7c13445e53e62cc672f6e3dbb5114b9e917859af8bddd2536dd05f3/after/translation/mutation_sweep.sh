#!/bin/bash
# Mutation sweep: inject a bug into a COPY of the Rust translation, build it as
# a standalone .so, and confirm the differential suite rejects it.
# Never touches translation/src/lib.rs.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ORIG="$ROOT/translation/src/lib.rs"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
cp "$ORIG" "$TMP/orig.rs"

# The interposer makes malloc request sizes observable; build it if missing.
if [ ! -f "$ROOT/translation/target/malloc_trace.so" ]; then
  mkdir -p "$ROOT/translation/target"
  cc -shared -fPIC -O2 -o "$ROOT/translation/target/malloc_trace.so" \
     "$ROOT/translation/tests/support/malloc_trace.c" -ldl || {
       echo "warning: could not build the allocator interposer; \
alloc-size mutants may survive"; }
fi

pass=0; fail=0; equiv=0
# Mutants that are PROVABLY equivalent at the API boundary, so surviving is the
# correct outcome. Each needs a justification, not just a listing.
#   "nul-skip: unconditional pos += 1"
#     After `pos += len`, `pos` is only ever (a) compared with `bufferSize` in
#     the outer guard and (b) used as `buffer + pos` in the NEXT iteration.
#     The `if` is false exactly when `pos >= bufferSize`, in which case the outer
#     guard fails and `pos` is never read again. So the extra increment is
#     unobservable. Confirmed empirically: 4.19M exhaustive input pairs do not
#     distinguish it.
is_expected_equivalent() {
  case "$1" in
    "nul-skip: unconditional pos += 1") return 0 ;;
    *) return 1 ;;
  esac
}
run() {
  local desc="$1"; shift
  cp "$TMP/orig.rs" "$TMP/mut.rs"
  "$@" "$TMP/mut.rs"
  if diff -q "$TMP/orig.rs" "$TMP/mut.rs" >/dev/null; then
    echo "SKIP (mutation did not apply): $desc"; return
  fi
  if ! rustc --edition 2021 --crate-type cdylib -O \
        -o "$TMP/libdriver_bad.so" "$TMP/mut.rs" 2>"$TMP/rustc.err"; then
    echo "SKIP (does not compile): $desc"; return
  fi
  local out rc
  out=$(cd "$ROOT/translation" && \
        MALLOC_TRACE_SO="$ROOT/translation/target/malloc_trace.so" \
        LD_PRELOAD="$ROOT/translation/target/malloc_trace.so" \
        RUST_DRIVER_SO="$TMP/libdriver_bad.so" \
        timeout 300 cargo test --release -- --test-threads=1 2>&1)
  rc=$?
  local summary
  summary=$(echo "$out" | grep -E '^test result:' | tail -1)
  if [ "$rc" -ne 0 ]; then
    # Non-zero exit means the suite rejected the mutant: either an assertion
    # fired, or the mutant crashed/aborted the harness (heap corruption, SIGSEGV
    # from an unchecked NULL, ...). Either way it is distinguishable from the C.
    local how="assert"
    if ! echo "$summary" | grep -q FAILED; then how="crash/abort"; fi
    echo "KILLED   ($how): $desc  [${summary:-no summary}]"; pass=$((pass+1))
  else
    if is_expected_equivalent "$desc"; then
      echo "EQUIV    : $desc  (provably unobservable at the API boundary)"
      equiv=$((equiv+1))
    else
      echo "SURVIVED : $desc  [$summary]"; fail=$((fail+1))
    fi
  fi
}

s() { sed -i "$1" "$2"; }

run "alloc: wrapping_mul -> saturating_mul"        s 's/numLines.wrapping_mul(/numLines.saturating_mul(/'
run "alloc: element size 8 -> 4"                   s 's/std::mem::size_of::<\*const \*const c_char>()/4usize/'
run "alloc: numLines -> numLines+1"                s 's/malloc(numLines.wrapping_mul/malloc((numLines+1).wrapping_mul/'
run "alloc: malloc -> calloc-like zero size"       s 's/malloc(numLines.wrapping_mul(std::mem::size_of::<\*const \*const c_char>()))/malloc(0)/'
run "guard: drop the NULL check"                   s 's/if buffer_ptrs.is_null() {/if false {/'
run "outer: line_index < numLines -> <="           s 's/while line_index < numLines \&\& pos < bufferSize/while line_index <= numLines \&\& pos < bufferSize/'
run "outer: pos < bufferSize -> pos <= bufferSize" s 's/while line_index < numLines \&\& pos < bufferSize/while line_index < numLines \&\& pos <= bufferSize/'
run "store: buffer+pos -> buffer"                  s 's/\*line_pointers.add(line_index) = buffer.wrapping_add(pos)/*line_pointers.add(line_index) = buffer/'
run "store: buffer+pos -> buffer+pos+1"            s 's/= buffer.wrapping_add(pos) as \*const c_char/= buffer.wrapping_add(pos + 1) as *const c_char/'
run "store: index line_index -> numLines-1-idx"    s 's/\*line_pointers.add(line_index) =/*line_pointers.add(numLines - 1 - line_index) =/'
run "inner: drop bounds guard"                     s 's/while (pos + len < bufferSize) \&\& \*buffer.wrapping_add(pos + len) != 0/while *buffer.wrapping_add(pos + len) != 0/'
run "inner: <= instead of <"                       s 's/while (pos + len < bufferSize) \&\&/while (pos + len <= bufferSize) \&\&/'
run "inner: signed compare > 0 (char sign bug)"    s 's/\*buffer.wrapping_add(pos + len) != 0 {/*buffer.wrapping_add(pos + len) > 0 {/'
run "advance: pos += len -> pos += len + 1"        s 's/^        pos += len;/        pos += len + 1;/'
run "advance: drop pos += len"                     s 's/^        pos += len;//'
run "nul-skip: unconditional pos += 1"             s 's/^        if pos < bufferSize {$/        if true {/'
run "nul-skip: never skip"                         s 's/^        if pos < bufferSize {$/        if false {/'
run "verify: drop line_index != numLines check"    s 's/if line_index != numLines {/if false {/'
run "verify: invert to =="                         s 's/if line_index != numLines {/if line_index == numLines {/'
run "verify: return dangling after free"           s 's/^        return std::ptr::null();$\n/        /'
run "zero-lines: special-case to NULL"             s 's|^    let mut line_index: usize = 0;|    if numLines == 0 { return std::ptr::null(); }\n    let mut line_index: usize = 0;|'
run "null-buffer: add a defensive NULL check"      s 's|^    let mut line_index: usize = 0;|    if buffer.is_null() { return std::ptr::null(); }\n    let mut line_index: usize = 0;|'
run "use Rust global allocator sizing (2x)"         s 's/numLines.wrapping_mul(std::mem::size_of::<\*const \*const c_char>())/numLines.wrapping_mul(16)/'

echo
echo "killed=$pass  known-equivalent=$equiv  UNEXPECTED-SURVIVORS=$fail"
[ "$fail" -eq 0 ] || exit 1
