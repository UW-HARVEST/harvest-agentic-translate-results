#!/usr/bin/env bash
# Mutation check: proves the differential suite is not vacuous.
#
# Each mutation is a plausible mistranslation applied to src/lib.rs. The suite
# MUST fail for every one of them, and must pass again once reverted.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

backup=$(mktemp)
cp src/lib.rs "$backup"
restore() { cp "$backup" src/lib.rs; rm -f "$backup"; }
trap restore EXIT

apply() { # apply <search> <replace> [occurrence-index]
    python3 - "$1" "$2" "${3:-0}" <<'PY'
import sys, pathlib
p = pathlib.Path("src/lib.rs"); s = p.read_text()
old, new, n = sys.argv[1], sys.argv[2], int(sys.argv[3])
if s.count(old) <= n:
    sys.exit(f"mutation target not found ({s.count(old)} occurrences): {old!r}")
i = -1
for _ in range(n + 1):
    i = s.index(old, i + 1)
p.write_text(s[:i] + new + s[i + len(old):])
PY
}

rc=0
check_fails() { # check_fails <description>
    local out
    out=$(timeout 600 cargo test --release 2>&1)
    if [[ $? -eq 0 ]]; then
        echo "NOT DETECTED: $1" >&2
        rc=1
    else
        echo "detected: $1"
        echo "$out" | grep -E '^test .* FAILED' | sed 's/^/      /'
    fi
}

echo "=== baseline: the unmutated translation must pass ==="
if timeout 600 cargo test --release > /dev/null 2>&1; then
    echo "baseline passes"
else
    echo "BASELINE FAILS -- fix that before interpreting mutations" >&2
    exit 1
fi

# --- 1. bad() reads the wrong stack slot --------------------------------------
cp "$backup" src/lib.rs
apply '"mov rax, qword ptr [rbp - 8]",' '"mov rax, qword ptr [rbp - 16]",' 1
check_fails "bad() reads [rbp-16] instead of [rbp-8]"

# --- 2. good() stores the wrong value -----------------------------------------
cp "$backup" src/lib.rs
apply '"mov dword ptr [rbp - 12], 5",' '"mov dword ptr [rbp - 12], 6",'
check_fails "good() stores 6 instead of 5"

# --- 3. driver tests all 64 bits of rdi instead of just edi -------------------
cp "$backup" src/lib.rs
apply '"mov dword ptr [rbp - 4], edi",' '"mov qword ptr [rbp - 16], rdi",'
apply '"cmp dword ptr [rbp - 4], 0",'  '"cmp qword ptr [rbp - 16], 0",'
check_fails "driver tests the full 64-bit rdi instead of edi"

# --- 4. driver's frame/call replaced by an idiomatic Rust if/else -------------
cp "$backup" src/lib.rs
apply '#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn driver' '#[cfg(any())]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn driver'
apply '#[cfg(not(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver' '#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver'
check_fails "driver written as an idiomatic Rust if/else (tail-calls bad)"

# --- 5. driver's branch inverted ---------------------------------------------
cp "$backup" src/lib.rs
apply '"je 2f",' '"jne 2f",'
check_fails "driver's branch condition inverted"

# --- 6. printIntPtrLine drops the newline ------------------------------------
cp "$backup" src/lib.rs
apply 'static FMT_D_NL: [u8; 4] = *b"%d\n\0";' 'static FMT_D_NL: [u8; 4] = *b"%d\0\0";'
check_fails "printIntPtrLine prints \"%d\" without the newline"

# --- 7. printIntPtrLine formats as unsigned ----------------------------------
cp "$backup" src/lib.rs
apply 'static FMT_D_NL: [u8; 4] = *b"%d\n\0";' 'static FMT_D_NL: [u8; 4] = *b"%u\n\0";'
check_fails "printIntPtrLine uses %u instead of %d"

# --- 8. an exported symbol removed -------------------------------------------
cp "$backup" src/lib.rs
apply '#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn good' '#[unsafe(naked)]
pub unsafe extern "C" fn good'
check_fails "good() loses its #[no_mangle] export"

# --- 9. eager binding restored (residue-visible linkage difference) ----------
cp "$backup" src/lib.rs
mv .cargo/config.toml .cargo/config.toml.disabled
# The rustflags change alters linkage, not source, and the harness's staleness
# check is mtime-based, so nudge the source to force the .so to be relinked.
touch src/lib.rs
check_fails "BIND_NOW instead of the C library's lazy binding"
mv .cargo/config.toml.disabled .cargo/config.toml
touch src/lib.rs

restore
trap - EXIT
echo
if (( rc == 0 )); then
    echo "ALL MUTATIONS DETECTED -- the suite is not vacuous"
    # Confirm the revert really restored a passing state.
    timeout 600 cargo test --release > /dev/null 2>&1 \
        && echo "post-revert baseline passes" \
        || { echo "post-revert baseline FAILS" >&2; rc=1; }
else
    echo "SOME MUTATIONS WENT UNDETECTED" >&2
fi
exit $rc
