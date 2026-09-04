# Differential verification log — `c_src/src/main.c` vs `translation/src/main.rs`

## How the two programs are run

| | command |
|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` → `./c_src/build/driver` |
| Rust | `cd translation && cargo build --release` → `./translation/target/release/driver` |

Both read a single `%d` from **stdin** and take no arguments. The test suite
(`translation/tests/differential.rs`) spawns both as subprocesses and compares
**stdout**, **stderr** and **exit status** for every input.

## Branches the C code actually has

`main` has exactly one decision, `if (x)`, on the value `scanf("%d", &x)` leaves
in `x` (initialised to `0`, and *not overwritten* when the conversion fails):

| input class | `x` | branch | stdout |
|---|---|---|---|
| empty / whitespace-only / EOF (input failure) | 0 (initialiser) | `bad()` | `0\n` |
| non-numeric first char, or sign then non-digit (matching failure) | 0 (initialiser) | `bad()` | `0\n` |
| parses to zero (`0`, `+0`, `-0`, `000`, `0abc`, `0.5`, …) | 0 | `bad()` | `0\n` |
| parses to non-zero | ≠0 | `good()` | `5\n` |
| out-of-`int` value whose low 32 bits are zero (`4294967296`, `8589934592`, `-4294967296`, …) | 0 | `bad()` | `0\n` |

Exit status is `0` and stderr is empty on **every** path — there is no error
path that writes to stderr and no non-zero `return` in the C source.

## Mismatches found

**None.** After building both programs, all enumerated input classes plus
~5,300 generated cases (every single byte 0x00–0xFF as the whole input, a sweep
of −300…300 in three whitespace variants, 1,500 numeric-alphabet fuzz cases and
1,000 raw-byte fuzz cases) produced byte-identical stdout, byte-identical stderr
and identical exit status. No change to `translation/src/main.rs` was required.

`c_src/` was not modified (only the untracked `c_src/build/` artefact directory
was created by CMake).

## Behaviours that were specifically checked, and why they are subtle

These are the places a translation would most plausibly drift. Each is covered
by a named test; all already agreed.

1. **`bad()` dereferences an uninitialised `int *` (CWE-457/824).**
   This is undefined behaviour, but the reference (unoptimised CMake default)
   build is deterministic: the stack slot holding `data` still contains the
   pointer `scanf("%d", &x)` was handed, i.e. `&main::x`, so the dereference
   prints the current value of `x`. `bad()` is only reachable when `x` is
   false, so the observable output is always `0\n`. Verified stable across 20+
   repeated runs and under every failing-parse input.
   The Rust side models this stale slot explicitly (`STALE_POINTEE`, set from
   `x` before the branch) instead of using `unsafe`, so it reproduces the same
   bytes without itself being UB.
   *Test:* `empty_input_takes_bad_branch`, `zero_takes_bad_branch`,
   `matching_failure_leaves_x_zero`, `output_shape_is_one_line_no_extra_bytes`.

2. **`scanf` reads across newlines, `fgets` does not.**
   `"\n\n\n\n5"` must reach `good()`, not fail. Leading whitespace is the full C
   set: space, `\t`, `\n`, `\v` (0x0b), `\f` (0x0c), `\r`.
   *Test:* `whitespace_is_skipped_across_newlines`.

3. **Failed conversion leaves `x` untouched rather than zeroing it.**
   Same observable value here (0) but for a different reason; a translation that
   returned an error and exited non-zero would diverge on exit status, which is
   why every assertion checks the status too.
   *Test:* `matching_failure_leaves_x_zero`, `sign_then_non_digit`.

4. **`%d` overflow is glibc's clamp-then-truncate, not saturation.**
   glibc accumulates into a `long`, clamps to `LONG_MIN`/`LONG_MAX` on `ERANGE`,
   then *stores into an `int`*, truncating to the low 32 bits. Consequences that
   a naive `i32::saturating` or `parse::<i32>()` implementation would get wrong:
   - `2147483648` → `INT_MIN` → **non-zero** → `5\n` (saturating to `INT_MAX`
     would coincidentally also print `5\n`, so this case alone is not decisive)
   - `4294967296` → `0` → **`bad()`** → `0\n` (saturation would print `5\n`)
   - `-4294967296`, `8589934592` → `0` → `0\n`
   - `9223372036854775808` → clamped to `LONG_MAX` → truncates to `-1` → `5\n`
   Confirmed against the C binary directly before accepting the Rust behaviour.
   *Test:* `overflow_truncation_and_signedness`.

5. **Conversion stops at the first non-digit and the remainder of stdin is
   never read.** `"0.5"`, `"1e5"`, `"3 4"`, `"0abc"` all convert only the
   leading integer. Nothing else in the program reads stdin, so no pushback is
   observable.
   *Test:* `trailing_junk_is_ignored`.

6. **`printf("%d\n", …)` output shape.** Exactly one line, no leading spaces, no
   padding, no second trailing newline, nothing on stderr.
   *Test:* `output_shape_is_one_line_no_extra_bytes`.

7. **Non-UTF-8 and NUL bytes on stdin.** The C code is byte-oriented; the Rust
   reader must not assume UTF-8 or treat `\0` as a terminator differently.
   *Test:* `non_utf8_input`, `deterministic_fuzz_raw_bytes`,
   `every_single_byte_input`.

8. **Very long digit runs.** 4 KiB and 200 KiB of `0`s (with and without a
   trailing `7`) cross stdio buffer boundaries and must not overflow or change
   the branch taken.
   *Test:* `long_digit_strings`.

9. **`argv` is ignored, and closed stdin behaves like empty stdin.**
   *Test:* `extra_argv_is_ignored`, `stdin_immediately_closed`.

## Result

`cargo test` in `translation/` passes in both debug and release profiles:
18 tests, 0 failed, 0 ignored, none disabled or skipped.
