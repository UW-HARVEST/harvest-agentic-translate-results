#!/usr/bin/env bash
# Sensitivity check for the differential test-suite.
#
# Injects a known bug into the Rust translation, one at a time, and asserts the
# suite FAILS. A test-suite that cannot detect a deliberately broken translation
# proves nothing, so this is what justifies the green runs.
#
# Mutations are literal (not regex) substitutions, applied by patch.py.
set -uo pipefail
cd "$(dirname "$0")"

BK=$(mktemp -d)
# Every file a mutation may touch has to be backed up, or a mutant leaks into the
# next case (which showed up as a "substring not found" for src/lib.rs).
SRCS=(src/prog.rs src/main.rs src/lib.rs)
for f in "${SRCS[@]}"; do cp "$f" "$BK/$(basename "$f")"; done
restore() { for f in "${SRCS[@]}"; do cp "$BK/$(basename "$f")" "$f"; done; }
verify_restored() {
  for f in "${SRCS[@]}"; do
    cmp -s "$BK/$(basename "$f")" "$f" || { echo "  !! $f was left mutated" >&2; return 1; }
  done
}
trap 'restore; rm -rf "$BK"; cargo build --offline >/dev/null 2>&1' EXIT

cat >"$BK/patch.py" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
if old not in s:
    sys.exit("substring not found: " + repr(old[:80]))
open(path, "w").write(s.replace(old, new, 1))
PY

pass=0; fail=0

# mutate <desc> <test-filter> <file> <old> <new>
mutate() {
  local desc="$1" filter="$2" file="$3" old="$4" new="$5"
  restore
  if ! python3 "$BK/patch.py" "$file" "$old" "$new"; then
    echo "  ?? MUTATION DID NOT APPLY: $desc" >&2; fail=$((fail+1)); return
  fi
  if ! cargo build --offline >/dev/null 2>&1; then
    echo "  ?? mutant does not compile: $desc" >&2; fail=$((fail+1)); return
  fi
  if timeout 600 cargo test --offline "$filter" >/dev/null 2>&1; then
    echo "  NOT CAUGHT  : $desc   (filter: $filter)"
    fail=$((fail+1))
  else
    echo "  caught      : $desc"
    pass=$((pass+1))
  fi
}

echo "== mutation sensitivity =="

mutate 'bad() prints a non-empty string instead of ""' \
  err_bad_uninitialised_via_exe src/prog.rs \
  'pub const BAD_DATA: &[u8] = b"";' \
  'pub const BAD_DATA: &[u8] = b"x";'

mutate 'good() payload changed' \
  cfg_good_direct src/prog.rs \
  'pub const GOOD_DATA: &[u8] = b"string";' \
  'pub const GOOD_DATA: &[u8] = b"strings";'

mutate 'printLine drops the NULL guard (prints for NULL too)' \
  err_printline_null src/prog.rs \
  'if let Some(line) = line {' \
  'if let Some(line) = Some(line.unwrap_or(b"")) {'

mutate 'printLine forgets the trailing newline' \
  cfg_printline_random_ascii src/prog.rs \
  'let _ = out.write_all(b"\n");' \
  ''

mutate 'printLine emits \r\n instead of \n' \
  err_printline_empty src/prog.rs \
  'let _ = out.write_all(b"\n");' \
  'let _ = out.write_all(b"\r\n");'

mutate 'scanf saturates to i32 instead of truncating' \
  err_scanf_int_truncation src/prog.rs \
  'Some(acc as i32)' \
  'Some(acc.clamp(i32::MIN as i64, i32::MAX as i64) as i32)'

mutate 'scanf wraps instead of clamping the long accumulator' \
  err_scanf_overflow_positive src/prog.rs \
  '            acc = match next {' \
  '            let next = Some(acc.wrapping_mul(10).wrapping_add(if negative { -digit } else { digit }));
            acc = match next {'

mutate 'scanf accepts a sign followed by a non-digit' \
  err_scanf_sign_then_nondigit src/prog.rs \
  '    if !cur.is_ascii_digit() {' \
  '    if false {'

# NOTE: whitespace-only inputs cannot discriminate this (a skip and a matching
# failure both end at x == 0); the exhaustive byte-classification row can.
mutate 'scanf does not treat \v and \f as whitespace' \
  cfg_exe_byte_classification src/prog.rs \
  "matches!(b, b' ' | b'\\t' | b'\\n' | b'\\x0b' | b'\\x0c' | b'\\r')" \
  "matches!(b, b' ' | b'\\t' | b'\\n' | b'\\r')"

mutate 'scanf only recognises + as a sign, not -' \
  cfg_exe_byte_classification src/prog.rs \
  "        b'-' | b'+' => {" \
  "        b'+' => {"

mutate 'scanf ignores the minus sign (negative overflow clamps the wrong way)' \
  err_scanf_overflow_negative src/prog.rs \
  "            let neg = cur == b'-';" \
  '            let neg = false;'

# NOTE: the program only ever observes *zero-ness* of x (`if (x) good(); else
# bad();`), so a value bug is visible only where it flips that bit - i.e. with a
# leading zero, or where truncation to int produces 0.
mutate 'scanf stops after the first digit' \
  cfg_exe_leading_zeros src/prog.rs \
  '            Some(b) if b.is_ascii_digit() => cur = b,' \
  '            Some(b) if b.is_ascii_digit() && false => cur = b,'

mutate 'scanf skips whitespace only once instead of in a run' \
  cfg_exe_leading_whitespace_kinds src/prog.rs \
  '            Some(b) if is_c_space(b) => continue,' \
  '            Some(b) if is_c_space(b) => match input.next_byte() {
                Some(b2) => break b2,
                None => return None,
            },'

mutate 'scanf consumes the terminating character instead of pushing it back' \
  cfg_so_main_repeated src/prog.rs \
  '''            Some(b) => {
                input.push_back(b);
                break;
            }''' \
  '''            Some(_) => {
                break;
            }'''

mutate 'scanf does not push back the offending char on a matching failure' \
  cfg_so_main_repeated src/prog.rs \
  '        input.push_back(cur);
        return None;' \
  '        return None;'

# NOTE: the *identity* of the pushed-back byte is not observable through any
# defined channel in this program - on a seekable fd the exit-time rewind hands
# the byte back to the file (so a correct implementation re-reads the same byte),
# and every path where the identity could matter is a matching failure, i.e. a
# `bad()` path whose C output is undefined. Only the push-back *count* is
# observable, which is what the two mutations above and this one exercise.
mutate 'the exit-time rewind is off by one byte' \
  cfg_so_main_repeated src/prog.rs \
  '        let back = self.unconsumed();' \
  '        let back = self.unconsumed() + 1;'

mutate 'stdin uses the 8192-byte Rust buffer instead of st_blksize' \
  cfg_exe_shared_stdin_leftover src/prog.rs \
  '                let b = m.blksize() as usize;' \
  '                let b = 8192usize; let _ = m.blksize();'

mutate 'stdin is not repositioned at exit' \
  cfg_exe_shared_stdin_leftover src/main.rs \
  '    input.reposition_if_seekable();' \
  ''

mutate 'the .so uses a fresh stdin per main() call' \
  cfg_so_main_repeated src/lib.rs \
  '    S.get_or_init(|| std::sync::Mutex::new(prog::CStdin::new()))' \
  '    let _ = &S;
    Box::leak(Box::new(std::sync::Mutex::new(prog::CStdin::new())))'

mutate 'main treats a matching failure as x = -1' \
  err_scanf_matching_failure src/prog.rs \
  '    if let Some(v) = scanf_int(input) {
        x = v;
    }' \
  '    x = scanf_int(input).unwrap_or(-1);'

mutate 'main inverts the zero test' \
  cfg_exe_single_digit src/prog.rs \
  '    if x != 0 {
        good(out);
    } else {
        bad(out);
    }' \
  '    if x == 0 {
        good(out);
    } else {
        bad(out);
    }'

mutate "the SIGPIPE disposition is left as Rust's SIG_IGN" \
  err_stdout_broken_pipe src/main.rs \
  '        signal(SIGPIPE, SIG_DFL);' \
  ''

restore
if ! verify_restored; then fail=$((fail+1)); fi
cargo build --offline >/dev/null 2>&1
echo
echo "caught $pass / $((pass+fail)) injected bugs"
[ "$fail" -eq 0 ] || exit 1
echo "OK: the suite detects every injected bug"
