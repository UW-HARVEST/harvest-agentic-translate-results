#!/usr/bin/env bash
# Proves the differential harness is not vacuous: each mutation below injects a
# deliberate translation bug into src/ and the run must FAIL. Sources are always
# restored (even on interrupt).
#
# This exists because an earlier version of the harness silently passed every
# mutation: `cargo test` compiles the library only as an `rlib`, so the
# `libdriver.so` that the tests dlopen was never refreshed. tests/common/mod.rs
# now rebuilds it and hard-fails on a stale artifact.
set -u -o pipefail
cd "$(dirname "$0")"

BAK=$(mktemp -d "${TMPDIR:-/tmp}/mutcheck.XXXXXX")
cp src/translated.rs src/lib.rs src/main.rs "$BAK/"
restore() { cp "$BAK/translated.rs" "$BAK/lib.rs" "$BAK/main.rs" src/; }
trap 'restore; rm -rf "$BAK"' EXIT INT TERM

patch_file() { # patch_file <file> <old> <new>
  python3 -c '
import sys
p, old, new = sys.argv[1:4]
s = open(p).read()
if old not in s:
    sys.exit("ANCHOR NOT FOUND in " + p)
open(p, "w").write(s.replace(old, new, 1))
' "$@"
}

FAILED=0
check() { # check <name> <test-target> <file> <old> <new>
  local name="$1" target="$2"; shift 2
  printf '%-58s' "$name"
  if ! patch_file "$@" >/dev/null 2>&1; then printf 'ERROR (anchor missing)\n'; FAILED=1; return; fi
  if timeout 600 cargo test --offline --test "$target" -- --test-threads=1 >/dev/null 2>&1; then
    printf 'NOT CAUGHT  <-- harness is blind here\n'; FAILED=1
  else
    printf 'caught by %s\n' "$target"
  fi
  restore
}

echo
echo "mutation                                                  result"
echo "----------------------------------------------------------------"

check "printIntLine off-by-one" print_functions src/translated.rs \
  'lock.write_all(int_number.to_string().as_bytes())' \
  'lock.write_all(int_number.wrapping_add(1).to_string().as_bytes())'

check "printLine drops the trailing newline" print_functions src/translated.rs \
  'let _ = lock.write_all(line);
        let _ = lock.write_all(b"\n");' \
  'let _ = lock.write_all(line);'

check "printLine ignores the NULL guard" print_functions src/lib.rs \
  'if line.is_null() {
        // C: `if (line != NULL)` -- nothing is printed for NULL.
        return;
    }' \
  'if line.is_null() {
        translated::print_line(Some(b"(null)"));
        return;
    }'

check "printLine lossy-converts bytes to UTF-8" print_functions src/lib.rs \
  'let bytes = std::ffi::CStr::from_ptr(line).to_bytes();
    translated::print_line(Some(bytes));' \
  'let owned = String::from_utf8_lossy(std::ffi::CStr::from_ptr(line).to_bytes()).into_owned();
    translated::print_line(Some(owned.as_bytes()));'

check "bad() prints a different element" subprocess_diff src/translated.rs \
  '            data[i] = source[i];
        }
        print_int_line(data[0]);
    }
}

/// `void good()`' \
  '            data[i] = source[i];
        }
        print_int_line(data[0] + 1);
    }
}

/// `void good()`'

check "scanf: saturate at INT_MAX instead of LONG_MAX" scanf_probe src/translated.rs \
  'let clamped: i64 = if value > saturated_hi {' \
  'let clamped: i64 = if value > i32::MAX as i128 { i32::MAX as i64 }
        else if value < i32::MIN as i128 { i32::MIN as i64 }
        else if value > saturated_hi {'

check "scanf: reject a leading '+'" scanf_probe src/translated.rs \
  "if b == b'+' || b == b'-' {" \
  "if b == b'-' {"

check "scanf: treat '\\r' as a non-space" scanf_probe src/translated.rs \
  "matches!(b, b' ' | b'\\t' | b'\\n' | 0x0b | 0x0c | b'\\r')" \
  "matches!(b, b' ' | b'\\t' | b'\\n' | 0x0b | 0x0c)"

check "executable no longer restores SIGPIPE to SIG_DFL" subprocess_diff src/main.rs \
  '    restore_default_sigpipe();' \
  '    #[allow(unused)] let _skip = restore_default_sigpipe;'

echo "----------------------------------------------------------------"
if ((FAILED)); then echo "RESULT: the harness missed at least one mutation."; exit 1; fi
echo "RESULT: every injected bug was detected."
