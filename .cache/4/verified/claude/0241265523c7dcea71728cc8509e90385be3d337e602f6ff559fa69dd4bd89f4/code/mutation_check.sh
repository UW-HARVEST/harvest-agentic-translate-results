#!/usr/bin/env bash
# Sensitivity check for the differential suite.
#
# Each mutation deliberately alters the Rust translation.  Mutations registered
# with `expect_caught` change *observable* behavior, so the named test target
# MUST fail; one that survives is a coverage hole.  Mutations registered with
# `expect_equivalent` are provably unobservable through this program's public
# behavior, so the suite is *expected* to keep passing — they are listed to
# document why, rather than left as mystery gaps.
#
# Usage: ./mutation_check.sh
set -u

ROOT="$(cd "$(dirname "$0")" && pwd)"
SRC="$ROOT/src/prog.rs"
LIB="$ROOT/src/lib.rs"
BACKUP_DIR="$(mktemp -d)"
cp "$SRC" "$BACKUP_DIR/prog.rs"
cp "$LIB" "$BACKUP_DIR/lib.rs"

restore() {
  cp "$BACKUP_DIR/prog.rs" "$SRC"
  cp "$BACKUP_DIR/lib.rs" "$LIB"
}
trap 'restore; rm -rf "$BACKUP_DIR"' EXIT

caught=0
missed=0
equivalent=0
broken=0

# $1 expectation (caught|equivalent), $2 description, $3 test target,
# $4 file to mutate, $5.. perl -0pi expressions
mutate() {
  local expect="$1"; shift
  local desc="$1"; shift
  local target="$1"; shift
  local file="$1"; shift
  restore
  for expr in "$@"; do
    perl -0pi -e "$expr" "$file"
  done
  if diff -q "$BACKUP_DIR/$(basename "$file")" "$file" >/dev/null; then
    echo "BROKEN    $desc — the pattern no longer matches the source"
    broken=$((broken + 1))
    return
  fi
  local ok=1
  local t
  for t in ${target//,/ }; do
    if ! timeout 600 cargo test --offline --quiet --test "$t" >/dev/null 2>&1; then
      ok=0
      break
    fi
  done
  if [ "$ok" -eq 1 ]; then
    if [ "$expect" = "equivalent" ]; then
      echo "equivalent  $desc  (unobservable, as documented)"
      equivalent=$((equivalent + 1))
    else
      echo "MISSED    $desc — '$target' still passed!"
      missed=$((missed + 1))
    fi
  else
    if [ "$expect" = "equivalent" ]; then
      echo "SURPRISE  $desc — expected to be unobservable, but '$target' failed"
      missed=$((missed + 1))
    else
      echo "caught      $desc  (by $target)"
      caught=$((caught + 1))
    fi
  fi
}

expect_caught()     { mutate caught "$@"; }
expect_equivalent() { mutate equivalent "$@"; }

echo "=== mutation sensitivity check ==="

# --- printLine / bad / good -------------------------------------------------

expect_caught "helperBad returns a live string instead of NULL" ffi_diff "$SRC" \
  's/let _ = char_string; \/\/ written to the stack frame, then abandoned\n    None/return Some(b"helperBad string");/'

expect_caught "helperGood1 loses its trailing-NUL trim (prints the NUL byte)" ffi_diff "$SRC" \
  's/Some\(&CHAR_STRING\[\.\.CHAR_STRING\.len\(\) - 1\]\)/Some(\&CHAR_STRING[..])/'

expect_caught "helperGood1 returns a different string" ffi_diff "$SRC" \
  's/helperGood1 string\\0/helperGood2 string\\0/'

expect_caught "printLine emits CRLF instead of LF" ffi_diff "$SRC" \
  's/out\.write_all\(b"\\n"\)/out.write_all(b"\\r\\n")/'

expect_caught "printLine emits no line terminator" ffi_diff "$SRC" \
  's/let _ = out\.write_all\(b"\\n"\);//'

expect_caught "printLine validates UTF-8 (lossy) instead of passing bytes through" ffi_diff "$SRC" \
  's/let _ = out\.write_all\(line\);/let _ = out.write_all(String::from_utf8_lossy(line).as_bytes());/'

expect_caught "printLine truncates its argument at 255 bytes" ffi_diff "$SRC" \
  's/let _ = out\.write_all\(line\);/let _ = out.write_all(\&line[..line.len().min(255)]);/'

expect_caught "printLine drops its NULL check" ffi_diff "$LIB" \
  's/if line\.is_null\(\) \{\n        return;\n    \}/if false { return; }/'

# --- scanf("%d") -----------------------------------------------------------

expect_caught "isspace() forgets the vertical tab" main_diff "$SRC" \
  's/c == 0x0B \|\| //'

expect_caught "isspace() forgets the form feed" main_diff "$SRC" \
  's/c == 0x0C \|\| //'

expect_caught "isspace() also accepts the NUL byte" main_diff "$SRC" \
  's/c == 0x20 \|\|/c == 0x00 || c == 0x20 ||/'

expect_caught "isdigit() is off by one at the top of the range" main_diff "$SRC" \
  "s/\\(b'0' as i32\\..=b'9' as i32\\)\\.contains\\(&c\\)/(b'0' as i32..=b'8' as i32).contains(\\&c)/"

expect_caught "strtol saturates negatives to LONG_MAX" main_diff "$SRC" \
  's/if negative \{\n            i64::MIN\n        \} else \{\n            i64::MAX\n        \}/i64::MAX/'

expect_equivalent "strtol uses the positive cutoff for negatives too (differs only at magnitude exactly 2^63, which saturates to LONG_MIN either way)" main_diff,sweep "$SRC" \
  's/i64::MIN\.unsigned_abs\(\)/i64::MAX as u64/'

expect_caught "the long->int narrowing is replaced by saturation" main_diff "$SRC" \
  's/Some\(strtol_base10\(&charbuf\) as i32\)/Some(strtol_base10(\&charbuf).clamp(i32::MIN as i64, i32::MAX as i64) as i32)/'

expect_caught "the 0x prefix is honored (base 16)" main_diff "$SRC" \
  's/\/\/ base is 10: not 0 \(would become 16\) and not 16, so no action\./c = input.inchar();/'

expect_caught "scanf does not skip leading whitespace" main_diff "$SRC" \
  's/if !is_space\(c\) \{\n            break;\n        \}/break;/'

expect_caught "the value test becomes x > 0 instead of x != 0" main_diff "$SRC" \
  's/if x != 0 \{/if x > 0 {/'

expect_caught "the pushed-back byte is dropped" main_diff "$SRC" \
  's/self\.pushed_back = Some\(c as u8\);/let _ = c;/'

expect_caught "digits are collected without an overflow guard" main_diff "$SRC" \
  's/Some\(v\) if v <= cutoff => acc = v,/Some(v) => acc = v,/'

expect_equivalent "the leading-zero branch drops the recorded zero (a leading zero never changes the value, and an emptied workspace yields x == 0 just like a matching failure)" main_diff,sweep "$SRC" \
  's/charbuf\.push\(c as u8\);\n        c = input\.inchar\(\);\n        if to_lower/c = input.inchar();\n        if to_lower/'

expect_caught "the sign is not recorded in the workspace" main_diff "$SRC" \
  "s/charbuf\\.push\\(c as u8\\);\\n        c = input\\.inchar\\(\\);\\n    \\}\\n\\n    \\/\\/ Leading base indication/c = input.inchar();\\n    }\\n\\n    \\/\\/ Leading base indication/"

expect_caught "main returns 1 instead of 0" main_diff "$SRC" \
  's/    \/\/ return 0;\n    0\n\}/    1\n}/'

expect_caught "stdin is read one byte at a time and EOF is sticky too early" main_diff "$SRC" \
  's/if self\.eof \{\n            return EOF;\n        \}/return EOF;/'

# --- Provably unobservable mutations ---------------------------------------
# `int x = 0;` runs *before* scanf, and scanf's return value is discarded, so
# "conversion failed, x untouched" and "conversion produced 0" are the same
# observable program.  These two mutants are therefore equivalent, not gaps.

expect_equivalent "a lone sign is accepted as the value 0" main_diff,sweep "$SRC" \
  "s/\\|\\| \\(charbuf\\.len\\(\\) == 1 && \\(charbuf\\[0\\] == b'\\+' \\|\\| charbuf\\[0\\] == b'-'\\)\\)//"

expect_equivalent "matching failure assigns 0 through the pointer anyway" main_diff,sweep "$SRC" \
  's/return None; \/\/ matching failure/return Some(0);/'

expect_equivalent "the stdin refill chunk shrinks to a single byte" main_diff,sweep "$SRC" \
  's/\[0u8; 4096\]/[0u8; 1]/'

restore
echo
echo "=== caught: $caught   equivalent: $equivalent   missed: $missed   broken patterns: $broken ==="
[ "$missed" -eq 0 ] && [ "$broken" -eq 0 ]
