# Differential verification of `c_src/src/main.c` vs `translation/src/main.rs`

Both programs are compared by **running them**, never by loading symbols:
`tests/differential.rs` spawns each binary as a subprocess, feeds identical
bytes on stdin, and asserts stdout, stderr, exit status *and* terminating
signal are identical.

## How to reproduce

```sh
# C reference
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
# -> c_src/build/driver

# Rust translation
cd translation && cargo build --release
# -> translation/target/release/driver

# Differential suite
cd translation && cargo test --release
```

## Mismatches found

### 1. `SIGPIPE` disposition — exit status divergence (FIXED)

**Symptom**

| stdin | stdout | C | Rust (before fix) |
|---|---|---|---|
| `5\n` | pipe whose reader closed | killed by `SIGPIPE`, shell status **141** | exited **0** |
| `abc\n` | pipe whose reader closed | status **141** | exited **0** |

Reproduced with:

```sh
echo 5 | c_src/build/driver              2>/dev/null | true   # -> 141
echo 5 | translation/target/release/driver 2>/dev/null | true   # -> 0
```

**Cause**

The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main` runs, so a
failing write returns `EPIPE` instead of terminating the process. A C program
launched from a shell inherits `SIG_DFL`, so glibc's exit-time flush of the
fully-buffered stdout is killed by `SIGPIPE`.

This is reachable because both programs buffer their entire output (437 bytes
on the accept path, well under both glibc's 4096-byte and Rust's 8192-byte
buffer) and emit it in a single write at exit, after the reader is already gone.

**Fix** — `translation/src/main.rs`: `restore_default_sigpipe()` calls
`signal(SIGPIPE, SIG_DFL)` as the first statement of `main`, declared as a bare
`extern "C"` so no new dependency is added. Both programs now report 141.

**Regression test** — `sigpipe_on_closed_stdout_matches`. Removing the fix makes
that test fail with `C=(None, Some(13)) Rust=(Some(0), None)`, confirming the
test detects the defect rather than merely passing.

## Divergence hazards checked that turned out to be correct already

Each of these was a plausible place for the translation to be wrong; each was
confirmed identical against the compiled C.

| Hazard | C behaviour | Confirmed |
|---|---|---|
| `fgets` vs `scanf` | `fgets` stops at the first `\n` and keeps it; never reads a second line | `5\n10\n` uses `5`, ignores `10` |
| Buffer geometry | `char in[100]`, so `fgets` reads at most **99** bytes | 99 nines accepted; 200 nines truncated to 99; 99 spaces then `7` never sees the digit |
| Truncation changes the value | 95 spaces + `12345` becomes 95 spaces + `1234` | parsed as `1234` in both |
| Immediate EOF | `fgets` returns `NULL`, leaving the `= ""` initialiser, so `strtol` performs no conversion | empty stdin takes the error path, exit 0 |
| Unreadable stdin | `fgets` returns `NULL`, same as EOF | stdin = `/` (EISDIR) and `/dev/null` both take the error path |
| `endp != str` | `strtol` sets `endptr` back to `str` on no conversion | `\n`, `   `, `abc`, `-`, `+`, `.5`, `--5`, `+-5`, `(5)`, `,1` all rejected |
| Partial conversion succeeds | `endp != str` is true if *any* digit was consumed | `12abc` → 12, `0x10` → 0, `1,000` → 1, `5e3` → 5 |
| `strtol` whitespace set | C locale: space `\t \n \v \f \r` | `\v5`, `\f5`, mixed runs all parse |
| Range check order | `endp != str && errno == 0 && INT_MIN <= tmp && tmp <= INT_MAX` | `2147483647` and `-2147483648` accepted; `2147483648`, `-2147483649` rejected |
| `long` vs `int` | `LONG_MAX`/`LONG_MIN` parse without `ERANGE` but fail the int range check | `9223372036854775807`, `-9223372036854775808` rejected |
| `ERANGE` | `strtol` returns `LONG_MAX`/`LONG_MIN` and sets `errno` | `9223372036854775808`, 26 nines rejected |
| Signed overflow in `add_bedrooms` | wraps (two's complement) as gcc emits | `2147483647` → `-2147483644` then `3`; matches byte for byte |
| `%.1f` | bathrooms only ever take 2.5 / 3.5 / 4.5, all exact in binary64 | no rounding-mode dependence exists on any input |
| Global state across two `run` calls | `the_house` persists, so floors 2→3→4 and bedrooms accumulate `2 * x` | 8-line output shape pinned by `accept_path_shape_is_eight_lines` |
| Embedded `NUL` | `fgets` copies it; `strtol` stops at it | `\x005\n` rejected, `5\x00abc\n` → 5 |
| Write error on stdout | glibc ignores the failed flush; `main` returns 0 | stdout = `/dev/full` gives exit 0 in both |
| Exit status | `main` always `return 0` | every input above exits 0 (except the SIGPIPE case) |

### Note on the `errno == 0` clause

Deleting the `ERANGE` check from the Rust `parse_val` does **not** change
observable behaviour, and the suite still passes. That is not a coverage gap:
glibc's `strtol` returns exactly `LONG_MAX` or `LONG_MIN` whenever it sets
`ERANGE`, and both are already outside `INT_MIN..=INT_MAX`, so the range check
subsumes the `errno` check. Verified with a standalone C probe over
`9223372036854775808`, `-9223372036854775809`, 20 nines and
`-0000009223372036854775808000`: every `ERANGE` case had
`in_int_range == 0`. The check is kept in the Rust for fidelity to the C.

## Evidence that the suite is sensitive, not merely green

Deliberate defects were injected into `translation/src/main.rs` one at a time
and the suite was re-run; the source was restored byte-identically afterwards.

| Injected defect | Result |
|---|---|
| `fgets` limit 100 instead of 99 | **caught** (`buffer_truncation_at_99_bytes`, `embedded_nul_bytes`) |
| call `run` once instead of twice | **caught** (12 of 18 tests failed) |
| `exit(1)` on the error path | **caught** (13 of 18 tests failed) |
| `%.1f` → `%.2f` | **caught** (12 of 18 tests failed) |
| drop the `ERANGE` check | not caught — provably equivalent, see note above |
| drop `restore_default_sigpipe()` | **caught** (`sigpipe_on_closed_stdout_matches`) |

## Coverage beyond the named tests

A randomized sweep of **3760** additional inputs (random byte strings over a
digit/sign/whitespace/NUL/high-byte alphabet up to 140 bytes, random 64-bit
decimal values with padding and junk tails, and every length 0..=129 of both
`9`-runs and space-runs followed by a digit) produced **0** mismatches on
stdout, stderr and exit status.

## Status

- Both programs build with no errors.
- `cargo test` passes in `translation/`: 20 tests, 0 failed, 0 ignored.
- No test is `#[ignore]`d, disabled or skipped. The two `#[cfg(unix)]` tests are
  platform guards for Unix-only signal and `/dev/full` semantics, not skips;
  they run and pass on this Linux host.
- `c_src/` sources are unmodified. `c_src/src/main.c` md5
  `830decf88cf7d3af6bdcbbfd902d7691`, `c_src/CMakeLists.txt` md5
  `02ba3005fed9b6d7d46c4fe335ac00d8`. The only addition under `c_src/` is the
  generated `build/` directory produced by the prescribed `cmake` commands.
