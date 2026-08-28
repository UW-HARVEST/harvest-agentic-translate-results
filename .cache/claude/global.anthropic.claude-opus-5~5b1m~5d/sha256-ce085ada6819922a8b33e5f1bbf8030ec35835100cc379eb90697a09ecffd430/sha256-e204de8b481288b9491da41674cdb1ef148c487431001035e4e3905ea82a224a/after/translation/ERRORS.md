# Differential verification log — `c_src/src/main.c` vs `translation/src/main.rs`

Method: build both programs, feed both the *same* bytes on stdin, and compare
stdout, stderr and exit status. Never loaded as a library; both are driven as
subprocesses (`translation/tests/differential.rs`).

* C (ground truth): `c_src/build/driver`
  — built with `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
* Rust: `translation/target/release/driver`
  — built with `cd translation && cargo build --release`

## Result

**No behavioral mismatch was found.** Across the 2,362 asserted cases in the
committed suite (162 hand-written + 2,200 seeded-random, in 20 test functions)
plus ~11,700 additional ad-hoc randomized inputs, stdout,
stderr and exit status were byte-identical in every case. The Rust source
required no behavioral change; the only edits made during this task were to
`translation/tests/differential.rs` (two compile errors in my own test
helpers, see "Test-harness defects" below).

Because nothing had to be fixed in the translation, the rest of this file
records the hazards that *were* specifically probed — the places where a
plausible translation would have diverged — together with the observed C
behavior and the reason the Rust code already matches it. These are the checks
a future reader should re-run after touching `src/main.rs`.

## Hazards verified (each would be a mismatch if translated naively)

### 1. `scanf("%d")` is a two-step conversion: saturate to `long`, then truncate to `int`

glibc converts the digit run with `strtol` (clamping at `LONG_MAX`/`LONG_MIN`
on overflow) and then *stores* that `long` into an `int`, discarding the high
bits. A one-step "parse as i32, clamp on overflow" translation gives different
answers for three distinct input ranges:

| stdin token | C `int` value | why |
|---|---|---|
| `5000000000` | `705032704` | fits in `long`, truncated to `int` |
| `2147483648` | `-2147483648` | fits in `long`, truncated to `int` |
| `9223372036854775807` (`LONG_MAX`) | `-1` | exact `long`, truncated |
| `9223372036854775808` | `-1` | saturates to `LONG_MAX`, truncated |
| `99999999999999999999` | `-1` | saturates to `LONG_MAX`, truncated |
| `-9223372036854775808` (`LONG_MIN`) | `0` | exact `long`, truncated |
| `-9223372036854775809` | `0` | saturates to `LONG_MIN`, truncated |

Note the asymmetry: saturating high yields `-1`, saturating low yields `0`.
`CStdin::scan_int` reproduces this by accumulating into `i64`, flipping a
`saturated` flag to `i64::MAX`/`i64::MIN`, then a final `as i32`.

Visible effect: `99999999999999999999` as the operation prints
`Error: Unknown operation -1`, while `-9223372036854775808` as the operation
selects `OP_COPY`. Covered by `scanf_long_boundary_saturation`.

### 2. `scanf` reads across newlines; the line structure is irrelevant

`6 2 3 1 2 3 0` and the same tokens one-per-line must behave identically. A
line-oriented (`fgets`/`read_line`) translation would break here. Also
covered: tabs, `\r\n`, vertical tab (`\v`), form feed (`\f`), 9,000 leading
spaces, 5,000 leading newlines, missing trailing newline, and extra trailing
tokens after the last needed value (silently ignored). Covered by
`scanf_whitespace_and_sign_handling`.

### 3. `scanf("%d")` stops at the first non-digit, and that is a *success*

`0x10` is not a hex literal for `%d`: glibc converts `0`, returns 1, and
leaves `x10` in the stream. So `6 1 / 2 0x10 5` reads byte 0 successfully and
then fails on byte 1 with `Error: Failed to read byte 1` — not byte 0.
Likewise `000000000-9` parses as `0` followed by `-9`. Covered by
`buffer_byte_read_failures` and `scanf_long_boundary_saturation`.

### 4. A bare sign, or a sign separated from its digits, is a matching failure

`-`, `+`, `-x` and `2 - 5 7` all fail conversion. `scan_int` consumes the sign
and then requires at least one digit (`any_digit`), returning `None`
otherwise. Covered by `op_read_failures`, `buffer_byte_read_failures`.

### 5. `int` → `size_t` sign extension in `buffer_split`

`main` reads `split_pos` as `int` and passes it to a `size_t` parameter. A
negative value becomes enormous, so the error message prints the *unsigned*
value:

```
Error: Split position 18446744073709551615 exceeds length 4
```

A translation that kept the value signed, or that clamped it, would print
something else (or wrongly succeed). Rust relies on `split_pos as usize`
sign-extending, exactly as the C conversion does. Note the interaction with
hazard 1: `-9223372036854775808` truncates to `int` `0` *first*, so it is a
legal split at position 0 and succeeds. Covered by `op_split` and
`scanf_long_boundary_saturation`.

### 6. `(uint8_t)byte` truncation of out-of-range bytes

Bytes are read as `int` and stored into `uint8_t`. `256` → `0`, `300` → `44`,
`-1` → `255`, `511` → `255`. No error is reported. Covered by
`byte_truncation_to_uint8` (which checks the truncation through `OP_REVERSE`
data output, not only through the checksum).

### 7. `positions += buf->length` in `buffer_rotate` mixes `int` and `size_t`

The C statement promotes `positions` to `size_t`, adds, and converts back to
`int`. For `length <= 256` that is a wrapping 32-bit add, which is what
`positions.wrapping_add(buf.length as i32)` does. `INT_MIN % length` also has
to truncate toward zero (C and Rust `%` agree). Verified with `-2147483648`,
`2147483647`, `LONG_MAX`, and saturating tokens over lengths 1, 3 and 256.
Covered by `op_rotate`.

### 8. `sum = (sum << 3) ^ data[i]` wraps mod 2^32

`uint32_t` shift discards high bits rather than overflowing. Rust's `u32 <<
3` does the same and does not panic (shift amount < 32), so this is identical
in debug and release. Verified over a 256-byte buffer and a run of twelve
`255`s. Covered by `op_checksum`.

### 9. Order of validation matters

The C reads the operation, then the count, then *all* buffers, and only then
dispatches on the operation. So an unknown operation still reports a bad
buffer count or a bad buffer first:

* `7 0` → `Error: Invalid buffer count 0` (not `Unknown operation 7`)
* `7 1 / 3 1 2` → `Error: Failed to read byte 2` (not `Unknown operation 7`)

Similarly `OP_SPLIT`/`OP_ROTATE` read their extra parameter *after* every
buffer, so a truncated buffer list is reported before the missing parameter.
Covered by `unknown_operations`, `op_split`, `op_rotate`.

### 10. `printf` formatting and exit status

`write_buffer` prints `%zu` for the length then `" %u"` per byte then `\n` —
so a zero-length buffer prints the single line `0`. `OP_SPLIT` always prints
*two* lines (one may be `0`). `OP_CHECKSUM` prints `%u\n` per buffer. Exit
status is `result != 0 ? 1 : 0`, and every error path returns 1. Covered
throughout; exit status is asserted on every case.

## Branch-coverage evidence (Phase C)

To confirm no reachable path was left untested, a **copy** of `c_src/src/main.c`
was compiled with `gcc --coverage` (in `scratch/cov/`; `c_src/` was not
modified) and the full committed corpus of 2,362 stdin inputs was replayed
through it.

Result: **every executable line inside `main()` is covered except line 461**,
and the only untaken branch directions anywhere in the reachable code are the
28 listed below. All are provably unreachable:

| C location | Untaken branch | Why unreachable |
|---|---|---|
| 67, 91, 97, 110, 120, 139, 161, 186, 217, 246, 391, 423, 460–461 | NULL / `malloc`-failure guards | every caller passes the address of a live object; the array is at most 100 × `sizeof(buffer_t)` ≈ 26 KB |
| 71 | `buf->length > 256` | `read_buffer` rejects any length > 256 before storing it |
| 76 | `Warning: Checksum mismatch` | `read_buffer` always stores a freshly computed checksum, and `buffer_copy` (via `OP_COPY`) is the only `validate_buffer` call `main` can reach |
| 85 | `initial_capacity <= 0` | `main` rejects `buffer_count <= 0` first |
| 125, 480 | `validate_buffer` failing inside `buffer_copy` | follows from 67 and 71 |
| 492, 549 | `buffer_reverse` / `buffer_rotate` returning non-zero | those functions only fail on a NULL buffer |
| 511 | `buffer_count >= 1` being false | `main` already guaranteed `buffer_count > 0` |

Three functions are dead code — never called from `main` at all:
`buffer_conditional_copy`, `buffer_copy_strided`, `process_buffer_array`. They
are translated (and kept behind `#[allow(dead_code)]`) for fidelity, but no
stdin can reach them, so they are deliberately not exercised by the suite.

The Rust translation keeps the reachable-but-unreachable-in-practice guards
(`length > 256`, the checksum warning, `initial_capacity <= 0`) so that the two
programs stay equivalent if the surrounding code ever changes. The `NULL`
checks are omitted because a Rust `&`/`&mut` reference cannot be null, which
makes those branches statically dead rather than merely unreached.

## The suite is not vacuous (mutation check)

A suite that passes because it asserts nothing useful is worthless, so two
deliberate mutants were injected into `src/main.rs`, confirmed to be caught,
and reverted:

| Mutant | Injected change | Result |
|---|---|---|
| 1 | `run` returns `0` instead of `1` on the error path — stdout and stderr stay *correct*, only the exit status is wrong | **9 of 20 tests fail**, all reporting `EXIT STATUS differs` |
| 2 | `scan_int` clamps to `i32::MIN`/`i32::MAX` in one step instead of saturating to `long` and then truncating (hazard 1 above) | **7 of 20 tests fail**, reporting `STDERR differs` |

Mutant 1 is the specific failure mode the task warns about: a suite that
checked only stdout would have passed it. Mutant 2 is the most plausible
real-world translation error in this program, and the suite localises it to
`scanf_long_boundary_saturation`, `count_range_validation`,
`buffer_length_validation`, `op_split`, `op_rotate`, `unknown_operations` and
`sweep_token_soup`.

## Test-harness defects found and fixed

These were bugs in my test code, not in the translation:

1. `Lcg::pick` was declared `fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T`,
   which made `rng.pick(SPACERS)` infer `T = str` and fail with `the size for
   values of type str cannot be known at compilation time` (E0277) plus an
   E0308. Changed to `fn pick<T: Copy>(&mut self, xs: &[T]) -> T` and dropped
   the now-redundant `*` derefs at the call sites.
2. `rng.pick(b"0123456789 \n\t-+")` passed a `&[u8; 15]` where `&[u8]` was
   wanted; fixed with `&b"..."[..]`.

One harness *design* hazard is worth flagging because it would have produced
spurious failures: the largest cases push ~100 KB into stdin and get ~100 KB
back out. Feeding stdin through a pipe from the test process and only then
reading stdout deadlocks once the 64 KB pipe buffer fills. The harness
therefore writes each input to a real file and hands that file to the child as
stdin, letting `Command::output()` drain stdout and stderr concurrently.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
cd ../../translation && cargo test --release        # 20 tests, 2,362 cases
```

`cargo test` (debug) passes as well; the suite uses `CARGO_BIN_EXE_driver`, so
it tests whichever profile it was built with. No test is `#[ignore]`d,
skipped or otherwise disabled.
