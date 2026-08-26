#!/bin/bash
# Sanity check that the differential test suite is not vacuous: inject a
# deliberate behavioural bug into the Rust translation, confirm the suite
# CATCHES it, then restore the original source.
#
# Usage: ./mutation_check.sh
set -u
cd "$(dirname "$0")" || exit 1

cp src/lib.rs "$TMPDIR/lib.rs.orig"
cp src/ffi.rs "$TMPDIR/ffi.rs.orig"
cp src/types.rs "$TMPDIR/types.rs.orig"

restore() {
    cp "$TMPDIR/lib.rs.orig" src/lib.rs
    cp "$TMPDIR/ffi.rs.orig" src/ffi.rs
    cp "$TMPDIR/types.rs.orig" src/types.rs
}
trap restore EXIT

fails=0
run_mutation() {
    local name="$1"; shift
    local file="$1"; shift
    local from="$1"; shift
    local to="$1"; shift
    local tests="$*"

    restore
    if ! MUT_FROM="$from" MUT_TO="$to" perl -0777 -pi \
        -e 'BEGIN { $f = $ENV{MUT_FROM}; $t = $ENV{MUT_TO} } s/\Q$f\E/$t/' "$file"; then
        echo "MUTATION SETUP FAILED: $name"; fails=$((fails+1)); return
    fi
    if ! grep -qF "$to" "$file"; then
        echo "MUTATION NOT APPLIED: $name"; fails=$((fails+1)); return
    fi
    # `cargo test` does not rebuild a cdylib-only lib: build it explicitly.
    if ! timeout 600 cargo build >"$TMPDIR/mut.log" 2>&1; then
        echo "MUTATION DID NOT COMPILE: $name"; fails=$((fails+1)); return
    fi
    # shellcheck disable=SC2086
    if timeout 600 cargo test $tests -- --test-threads=1 >"$TMPDIR/mut.log" 2>&1; then
        echo "!! NOT CAUGHT: $name  (tests: $tests)"
        fails=$((fails+1))
    else
        local n
        n=$(grep -c '^test .* FAILED' "$TMPDIR/mut.log")
        echo "caught: $name ($n failing test(s))"
    fi
}

run_mutation "status bit-field initialised to 14 instead of 15" \
    src/lib.rs "flags.set_status(15);" "flags.set_status(14);" \
    "--test valid_paths"

run_mutation "float->int cast saturates (Rust) instead of cvttss2si (gcc)" \
    src/ffi.rs \
    "if value.is_nan() || value >= 2147483648.0f32 || value < -2147483648.0f32 {
        c_int::MIN
    } else {
        value as c_int
    }" \
    "value as c_int" \
    "--test error_paths --test valid_paths"

run_mutation "mode taken from the wrong bit position of param" \
    src/lib.rs \
    "flags.set_mode(((param >> 3) & 0x7) as u32);" \
    "flags.set_mode(((param >> 4) & 0x7) as u32);" \
    "--test valid_paths"

run_mutation "mode accessor wired to the status bit-field" \
    src/types.rs \
    "mode / set_mode => MODE," \
    "mode / set_mode => STATUS," \
    "--test valid_paths"

run_mutation "wrong magic constant in confuse_types op 0" \
    src/lib.rs \
    "(*state).data.int_val = 1078530011;" \
    "(*state).data.int_val = 1078530012;" \
    "--test valid_paths"

run_mutation "union bytes treated as unsigned char" \
    src/lib.rs \
    "result = (bytes[0] as c_int).wrapping_add(bytes[1] as c_int);" \
    "result = (bytes[0] as u8 as c_int).wrapping_add(bytes[1] as u8 as c_int);" \
    "--test valid_paths"

run_mutation "counter incremented by 2 instead of 1" \
    src/lib.rs \
    "let next_counter = (flags.counter().wrapping_add(1)) & 0x1F;" \
    "let next_counter = (flags.counter().wrapping_add(2)) & 0x1F;" \
    "--test valid_paths"

run_mutation "snprintf given capacity-1 instead of capacity" \
    src/lib.rs \
    "        capacity as isize as usize,
        cstr(FMT_STATE_MODE)," \
    "        (capacity as isize as usize).saturating_sub(1),
        cstr(FMT_STATE_MODE)," \
    "--test error_paths"

run_mutation "LOG_OPERATION prints count-1" \
    src/lib.rs \
    "ffi::printf(cstr(FMT_LOG_MEMCHR_FOUND), count);" \
    "ffi::printf(cstr(FMT_LOG_MEMCHR_FOUND), count - 1);" \
    "--test valid_paths"

run_mutation "uint printed with %d instead of %u" \
    src/lib.rs \
    'b"Read as uint: %u' \
    'b"Read as uint: %d' \
    "--test valid_paths"

run_mutation "search byte derived with %9 instead of %10" \
    src/lib.rs \
    "let search_char = (b'0' as c_int).wrapping_add(param3 % 10) as c_char;" \
    "let search_char = (b'0' as c_int).wrapping_add(param3 % 9) as c_char;" \
    "--test valid_paths"

run_mutation "capacity zero-extended instead of sign-extended for malloc" \
    src/lib.rs \
    "(*state).buffer = ffi::malloc(capacity as isize as usize) as *mut c_char;" \
    "(*state).buffer = ffi::malloc(capacity as u32 as usize) as *mut c_char;" \
    "--test error_paths"

run_mutation "process_buffer returns 0 instead of -1 on NULL" \
    src/lib.rs \
    "        ffi::printf(cstr(FMT_ERR_NULL_PROCESS_BUFFER));
        return -1;" \
    "        ffi::printf(cstr(FMT_ERR_NULL_PROCESS_BUFFER));
        return 0;" \
    "--test error_paths"

run_mutation "out-of-range operation falls through to case 0" \
    src/lib.rs \
    "        _ => {}" \
    "        _ => { (*state).data.int_val = 1078530011; }" \
    "--test error_paths"

restore
timeout 600 cargo build >"$TMPDIR/mut.log" 2>&1
echo
if [ "$fails" -eq 0 ]; then
    echo "MUTATION CHECK PASSED: every injected bug was detected."
else
    echo "MUTATION CHECK FAILED: $fails mutation(s) went undetected."
fi
exit "$fails"
