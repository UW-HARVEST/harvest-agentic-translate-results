#!/usr/bin/env bash
# Test-suite sensitivity check ("are the differential tests able to see a bug?").
#
# For each mutation: inject it into src/lib.rs, verify it really landed, run the
# differential suite, and require the suite to FAIL. Any mutation the suite does
# not catch is a blind spot. src/lib.rs is restored from .pristine/lib.rs
# afterwards (and on any interrupt).
#
# Usage:  ./mutation_check.sh [profile]        profile = debug | release (default both)
set -u
cd "$(dirname "$0")"

SRC=src/lib.rs
PRISTINE=.pristine/lib.rs
LOGDIR=.mutation-logs

if [ ! -f "$PRISTINE" ]; then
    echo "FATAL: $PRISTINE missing. Create it from a known-good src/lib.rs first." >&2
    exit 1
fi
mkdir -p "$LOGDIR"

restore() { cp "$PRISTINE" "$SRC"; }
trap restore EXIT INT TERM

PROFILES=${1:-"debug release"}

# ---------------------------------------------------------------------------
# Mutants that are PROVABLY semantically equivalent to the original and must
# therefore NOT make the suite fail. Verified by exhaustive enumeration over
# every NUL mask for bufferSize <= 14 crossed with numLines <= 17 (589,806
# inputs, 0 divergences) -- see the note in VERIFICATION.md.
#   * unconditional-terminator-skip: after `pos += len`, `pos <= bufferSize`
#     always holds; when `pos == bufferSize` the extra `pos += 1` cannot be
#     observed because the outer guard `pos < bufferSize` already fails.
#   * inner-bound-off-by-one-other-way: stopping the scan one byte early only
#     shortens `len` (which is not observable) in exactly the cases where the
#     subsequent `if pos < bufferSize { pos += 1 }` adds the byte back.
# ---------------------------------------------------------------------------
EQUIVALENT="unconditional-terminator-skip inner-bound-off-by-one-other-way"

is_equivalent() {
    case " $EQUIVALENT " in *" $1 "*) return 0;; esac
    return 1
}

# ---------------------------------------------------------------------------
# mutations: name | python-literal search | python-literal replacement
# ---------------------------------------------------------------------------
mutations() {
cat <<'EOF'
wrong-sizeof|core::mem::size_of::<*const c_char>()|4usize
oversized-sizeof|core::mem::size_of::<*const c_char>()|16usize
no-free-on-failure|        free(buffer_ptrs);\n        return core::ptr::null();|        return core::ptr::null();
unconditional-terminator-skip|        if pos < buffer_size {\n            pos += 1;|        if true {\n            pos += 1;
inner-bound-off-by-one|while (pos + len < buffer_size)|while (pos + len <= buffer_size)
inner-bound-off-by-one-other-way|while (pos + len < buffer_size)|while (pos + len + 1 < buffer_size)
outer-numlines-bound|while line_index < num_lines|while line_index <= num_lines
outer-bufsize-bound|&& pos < buffer_size {|&& pos <= buffer_size {
drop-malloc-null-check|    if buffer_ptrs.is_null() {\n        return core::ptr::null();\n    }|
drop-count-reconciliation|if line_index != num_lines {|if false {
count-reconciliation-lt|if line_index != num_lines {|if line_index > num_lines {
split-on-newline|.read() != 0|.read() != 10i8
pointer-off-by-one|.write(buffer.wrapping_add(pos) as *const c_char)|.write(buffer.wrapping_add(pos + 1) as *const c_char)
skip-two-past-terminator|            pos += 1; /* Skip|            pos += 2; /* Skip
saturating-mul|num_lines.wrapping_mul(|num_lines.saturating_mul(
checked-mul-panic|malloc(num_lines.wrapping_mul(core::mem::size_of::<*const c_char>()))|malloc(num_lines.checked_mul(core::mem::size_of::<*const c_char>()).unwrap_or(usize::MAX))
use-rust-allocator|malloc(num_lines.wrapping_mul(core::mem::size_of::<*const c_char>()))|{ let n = num_lines.wrapping_mul(core::mem::size_of::<*const c_char>()); if n == 0 { malloc(0) } else { std::alloc::alloc(std::alloc::Layout::from_size_align(n, 8).unwrap()) as *mut c_void } }
len-starts-at-one|        let mut len: usize = 0;|        let mut len: usize = 1;
skip-guard-off-by-one|        if pos < buffer_size {|        if pos + 1 < buffer_size {
COVERAGE-large-buffer|while (pos + len < buffer_size)|while (pos + len < buffer_size.min(200))
COVERAGE-large-numlines|            .add(line_index)|            .add(if line_index > 500 { line_index - 1 } else { line_index })
COVERAGE-empty-line|        let mut len: usize = 0;|        let mut len: usize = 0;\n        if pos < buffer_size && buffer.wrapping_add(pos).read() == 0 { pos += 1; continue; }
COVERAGE-high-bit|while (pos + len < buffer_size) && buffer.wrapping_add(pos + len).read() != 0|while (pos + len < buffer_size) && buffer.wrapping_add(pos + len).read() > 0
COVERAGE-zero-numlines|    let mut line_index: usize = 0;|    if num_lines == 0 { return core::ptr::null(); }\n    let mut line_index: usize = 0;
EOF
}

apply() { # $1=search $2=replace ; returns 2 if pattern not found
    python3 - "$SRC" "$1" "$2" <<'PY'
import sys
path, frm, to = sys.argv[1], sys.argv[2], sys.argv[3]
frm = frm.encode().decode('unicode_escape')
to  = to.encode().decode('unicode_escape')
s = open(path).read()
if frm not in s:
    sys.stderr.write("PATTERN-NOT-FOUND: %r\n" % frm)
    sys.exit(2)
open(path, "w").write(s.replace(frm, to, 1))
PY
}

for profile in $PROFILES; do
  echo "######## profile: $profile ########"
  relflag=""
  [ "$profile" = "release" ] && relflag="--release"

  caught=(); missed=(); broken=(); equiv_ok=(); equiv_bad=()

  while IFS='|' read -r name frm to; do
    [ -z "${name:-}" ] && continue
    cp "$PRISTINE" "$SRC"
    if ! apply "$frm" "$to" 2>"$LOGDIR/$name.apply.err"; then
        echo "  BROKEN  $name : mutation could not be applied ($(cat "$LOGDIR/$name.apply.err"))"
        broken+=("$name"); continue
    fi
    # Prove the mutation is really in the file we are about to compile.
    if cmp -s "$PRISTINE" "$SRC"; then
        echo "  BROKEN  $name : file unchanged after 'successful' mutation"
        broken+=("$name"); continue
    fi

    log="$LOGDIR/$name.$profile.log"
    timeout 600 cargo test $relflag --tests >"$log" 2>&1
    rc=$?

    if grep -q "STALE ARTIFACT" "$log"; then
        echo "  BROKEN  $name : harness reported a stale artifact"
        broken+=("$name"); continue
    fi
    # A real compile failure says "could not compile" / "error[EXXXX]".
    if grep -qE "could not compile|^error\[E[0-9]+\]" "$log"; then
        echo "  BROKEN  $name : does not compile ($(grep -m1 -E '^error' "$log" | cut -c1-90))"
        broken+=("$name"); continue
    fi

    if is_equivalent "$name"; then
        if [ $rc -eq 0 ]; then
            echo "  equiv   $name : suite passed, as required for an equivalent mutant"
            equiv_ok+=("$name")
        else
            why=$(grep -E "^test .*FAILED|SIGSEGV|signal" "$log" | head -2 | tr '\n' ' ')
            echo "  FLAKY   $name : suite FAILED on a provably equivalent mutant $why"
            equiv_bad+=("$name")
        fi
        continue
    fi

    if [ $rc -ne 0 ]; then
        why=$(grep -E "^test .*FAILED|SIGSEGV|signal" "$log" | head -2 | tr '\n' ' ')
        echo "  caught  $name (rc=$rc) $why"
        caught+=("$name")
    else
        echo "  MISS    $name : suite still PASSED  <-- BLIND SPOT"
        missed+=("$name")
    fi
  done < <(mutations)

  cp "$PRISTINE" "$SRC"
  echo
  echo "  caught      (${#caught[@]}): ${caught[*]:-<none>}"
  echo "  MISSED      (${#missed[@]}): ${missed[*]:-<none>}"
  echo "  equiv ok    (${#equiv_ok[@]}): ${equiv_ok[*]:-<none>}"
  echo "  FALSE-POS   (${#equiv_bad[@]}): ${equiv_bad[*]:-<none>}"
  echo "  broken      (${#broken[@]}): ${broken[*]:-<none>}"
  if [ ${#missed[@]} -eq 0 ] && [ ${#equiv_bad[@]} -eq 0 ] && [ ${#broken[@]} -eq 0 ]; then
    echo "  ==> profile $profile: suite is SENSITIVE and PRECISE"
  else
    echo "  ==> profile $profile: PROBLEM (see above)"
  fi
  echo
done

restore
echo "src/lib.rs restored from $PRISTINE"
cmp -s "$PRISTINE" "$SRC" && echo "restore verified: identical"
