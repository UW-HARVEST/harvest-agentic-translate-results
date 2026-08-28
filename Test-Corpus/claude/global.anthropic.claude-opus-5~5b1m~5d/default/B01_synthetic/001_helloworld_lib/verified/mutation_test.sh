#!/usr/bin/env bash
# Meta-verification: proves the differential suite is not vacuous.
#
# Each mutant injects a change into src/hello.rs, rebuilds the cdylib and reruns
# the suite:
#   KILL  mutants change observable behaviour and MUST make the tests fail.
#   EQUIV mutants are observationally identical to the original and MUST still
#         pass — they catch a suite that reports false divergences.
#
# src/hello.rs is always restored, including on interrupt.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

ORIG=$(mktemp)
cp src/hello.rs "$ORIG"
LOG=$(mktemp)
restore() { cp "$ORIG" src/hello.rs; }
cleanup() { restore; cargo build --offline >/dev/null 2>&1; rm -f "$ORIG" "$LOG"; }
trap cleanup EXIT INT TERM

wrong=0
total=0

run_mut () {
  local expect="$1" desc="$2"
  total=$((total+1))

  # The rebuild is essential: `cargo test` does not build a cdylib-only lib
  # target, so without this the mutant would never reach the .so under test.
  if ! cargo build --offline >"$LOG" 2>&1; then
    printf '  \033[31mBUILD FAIL\033[0m  %s (did not compile; not a behavioural test)\n' "$desc"
    wrong=$((wrong+1)); restore; return
  fi

  cargo test --offline --test phase_b --test phase_c --test smoke >"$LOG" 2>&1
  local rc=$? nfail got
  nfail=$(grep -cE '\.\.\. FAILED' "$LOG")
  [ "$rc" -ne 0 ] && got=KILL || got=EQUIV

  if [ "$got" = "$expect" ]; then
    if [ "$expect" = KILL ]; then
      printf '  \033[32mKILLED\033[0m    (%2d failing) %s\n' "$nfail" "$desc"
    else
      printf '  \033[32mPASSED\033[0m    (equivalent) %s\n' "$desc"
    fi
  else
    wrong=$((wrong+1))
    if [ "$expect" = KILL ]; then
      printf '  \033[31mSURVIVED\033[0m   <-- BLIND SPOT: %s\n' "$desc"
    else
      printf '  \033[31mFALSE FAIL\033[0m <-- %s (%d failing)\n' "$desc" "$nfail"
    fi
  fi
  restore
}

py () { python3 -c "
import sys
p='src/hello.rs'; s=open(p).read()
old,new=sys.argv[1],sys.argv[2]
assert old in s, 'mutation pattern not found: '+repr(old)
open(p,'w').write(s.replace(old,new,1))
" "$1" "$2" || { echo "  MUTATION SETUP FAILED"; wrong=$((wrong+1)); }; }

CALL='        c_printf(FORMAT.as_ptr() as *const c_char);'
FMT='&[u8; 14] = b"Hello World!\n\0"'

echo "Mutation testing the Rust translation of c_src/src/hello.c"
echo

# ---- observable divergences: must be detected -----------------------------
py "$FMT" '&[u8; 14] = b"Hello world!\n\0"'   ; run_mut KILL  "message text: 'World' -> 'world'"
py "$FMT" '&[u8; 14] = b"Hello World!\0\0"'   ; run_mut KILL  "drop the trailing newline"
py "$FMT" '&[u8; 14] = b"Hello World!\r\n"'   ; run_mut KILL  "CRLF instead of LF"
py "$FMT" '&[u8; 15] = b"Hello  World!\n\0"'  ; run_mut KILL  "double space inside the message"
py "$FMT" '&[u8; 13] = b"Hello World\n\0"'    ; run_mut KILL  "drop the exclamation mark"
py "$CALL" ''                                 ; run_mut KILL  "emit nothing at all"
py "$CALL" "$CALL"$'\n'"$CALL"                ; run_mut KILL  "emit the line twice"
py '    0
}' '    1
}'                                            ; run_mut KILL  "return 1 instead of 0"
py '    0
}' '    -1
}'                                            ; run_mut KILL  "return -1 instead of 0"
py "$CALL" '        return c_printf(FORMAT.as_ptr() as *const c_char);' \
                                              ; run_mut KILL  "propagate printf's return value"
py "$CALL" '        if c_printf(FORMAT.as_ptr() as *const c_char) < 0 { return -1; }' \
                                              ; run_mut KILL  "report I/O failure instead of swallowing it"
py "$CALL" '        if c_printf(FORMAT.as_ptr() as *const c_char) < 0 { panic!("io"); }' \
                                              ; run_mut KILL  "panic on I/O failure"
py "$CALL" '        let _ = FORMAT; print!("Hello World!\n");' \
                                              ; run_mut KILL  "write via Rust stdout, not the C stream"
py "$CALL" '        let _ = FORMAT; c_printf(c"Hello World!\n".as_ptr()); c_printf(c" ".as_ptr());' \
                                              ; run_mut KILL  "one extra space byte"
py '#[unsafe(no_mangle)]' '#[unsafe(export_name = "helloworld_")]' \
                                              ; run_mut KILL  "rename the exported symbol"

# ---- observationally equivalent: must still pass --------------------------
py "$FMT" '&[u8; 14] = b"Hello World\x21\n\0"'; run_mut EQUIV '\x21 escape for the "!"'
py "$CALL" '        let _ = FORMAT; c_printf(c"Hello World!\n".as_ptr()); c_printf(c"".as_ptr());' \
                                              ; run_mut EQUIV 'extra printf("") writing zero bytes'
py "$CALL" '        c_printf(c"%s".as_ptr(), FORMAT.as_ptr());' \
                                              ; run_mut EQUIV "same bytes through a %s format"

echo
if [ "$wrong" -eq 0 ]; then
  printf '\033[32mAll %d mutants behaved as expected: the suite detects every observable\n' "$total"
  printf 'divergence and reports no false divergences.\033[0m\n'
else
  printf '\033[31m%d of %d mutants did NOT behave as expected.\033[0m\n' "$wrong" "$total"
fi
exit "$wrong"
