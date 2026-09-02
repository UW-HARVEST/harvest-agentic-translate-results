# Differential verification of the C → Rust translation

Reference: `c_src/src/main.c` + `c_src/src/lib.c`, built with CMake to
`c_src/build/driver`.
Translation: `translation/src/main.rs` + `translation/src/lib.rs`, built with
`cargo build --release` to `translation/target/release/driver`.

Both programs are driven **as executables**: stdin is piped in, and stdout,
stderr and the exit status are compared byte for byte. The Rust code is never
loaded as a library. The harness lives in `tests/differential.rs`.

## Result

**No behavioural mismatch was found.** Every input class enumerated below
produces identical stdout, identical stderr and an identical exit status.

This document records what was checked rather than a list of repairs, because
no repair to `translation/` was required. The sections below name the places
where a mismatch was plausible, what the C actually does there, and how it was
confirmed — so the next reader can re-check the same ground instead of taking
"it passed" on trust.

## How much was compared

| Sweep | Cases | Mismatches |
|---|---|---|
| `cargo test` suite (24 tests, `tests/differential.rs`) | ~34,000 stdin streams | 0 |
| Exhaustive y/n strings, length 0–13, operations 2 and 3 | 32,766 | 0 |
| Random y/n strings, length 14–80, operations 0–3 | 6,000 | 0 |
| `atoi`/`strtol` boundary matrix on both numeric lines | 16,625 | 0 |
| Random raw byte streams (whole stdin is garbage) | 2,700 | 0 |
| Structured + boundary-length sweeps around the 1024-byte buffer | ~55,000 | 0 |

No test is `#[ignore]`d, disabled or skipped. Nothing in `c_src/` was modified;
the harness builds the C program out-of-tree into `translation/target/c_build`
when `c_src/build/driver` is absent, so a test run never writes into `c_src/`.

## Behaviours that had to match exactly, and were verified

### 1. `fgets`, not `scanf` — line-oriented reads with a hard 1024-byte cap

`main` calls `fgets(input_buffer, 1024, stdin)` three times. `fgets` stops
after at most 1023 bytes **and leaves the remainder in the stream**. So a first
line of 1030 `0` characters is consumed as the *operation* (first 1023 bytes)
and its tail becomes the *parameter* line. A naive `read_line` translation
would consume the whole line and shift every subsequent read.

`main.rs` reimplements `fgets` over a `BufRead` with `fill_buf`/`consume`, so
unread bytes stay queued. Confirmed by
`lines_longer_than_the_1024_byte_buffer`, which walks lengths 1020–1026 and
2048 for the decision line and 1030 for the operation and parameter lines.

### 2. Three distinct EOF error paths, each exiting 1

Each `fgets` returning `NULL` prints a *different* message to **stderr** and
returns 1:

- no operation line → `Error reading operation\n`
- no parameter line → `Error reading parameter\n`
- no decision line → `Error reading decision string\n`

A stdout-only comparison would pass while the exit status diverged, so
`assert_same` asserts all three channels. Confirmed by
`stdin_truncated_at_every_fgets` for stdin of `""`, `"0\n"`, `"0"`, `"0\n0\n"`
and `"0\n0"`. Note `"0"` (no newline) is a *successful* `fgets`, so it fails at
the second read, not the first.

### 3. `atoi` is `(int) strtol(..., 10)` — it saturates, then truncates

This was the highest-risk spot. glibc's `atoi` skips leading whitespace,
accepts one sign, stops at the first non-digit, **saturates at `long` bounds**
(64-bit here) and then **truncates to `int`**. Consequences that a plain
`str::parse::<i32>()` would get wrong:

- `"4294967296"` → `0` (truncation), not a parse error
- `"9223372036854775808"` → saturates to `LONG_MAX`, truncates to `-1`
- `"-9223372036854775809"` → saturates to `LONG_MIN`, truncates to `0`
- `"12abc"` → `12`; `"abc"` → `0`; `"0x1f"` → `0`; `"3.99"` → `3`; `"007"` → `7`
- `""` and `" "` → `0`, which selects operation 0, not an error

`atoi` in `main.rs` accumulates in `i64` with `checked_mul`/`checked_add`,
latches an overflow flag, saturates to `i64::MIN`/`MAX`, then casts with `as
i32`. Confirmed by `atoi_saturates_like_strtol_then_truncates_to_int` (a full
23×23 matrix of boundary values across both numeric lines) and
`atoi_parsing_of_operation_and_parameter`.

### 4. Only a *trailing* newline is stripped, and `strlen` defines the length

`main` computes `len = strlen(input_buffer)` and strips one trailing `'\n'`.
Two consequences:

- An embedded NUL byte truncates the decision string. `"y\0nn"` has `len == 1`,
  so operation 2 sees a single decision and returns `1001`, not `1002`.
- A `\r\n` line ending leaves the `\r` in place as a decision character, where
  `parse_bool` maps it to `false`.

Confirmed by `embedded_nul_bytes_truncate_the_decision_string` and the CRLF
cases in `blank_lines_and_missing_newlines`.

### 5. `apply_permissions` has a fall-through that looks like a bug

```c
} else if (read && write) {
    if (permission_value == 6) {
        return 50 + permission_value;
    }
}                      /* no else -- falls through to `return 0` */
```

`lib.rs` reproduces this: the `read && write` arm returns `56` only when
`permission_value == 6` and otherwise falls out of the chain to the final
`return 0`. (The condition is in fact always true on that arm, since
`read && write && !execute` forces `permission_value == 6`, so the
fall-through is dead — but the structure is preserved rather than
"simplified".) The observed output set for operation 0 is exactly
`{-20, -10, -2, -1, 0, 14, 23, 35, 56, 107}` for both programs.

### 6. `validate_sequence` aliases the caller's `char *` as `bool *`

```c
bool *bools = (bool*)sequence;   /* Reuse buffer */
for (size_t i = 0; i < len; i++) {
    bool val = parse_bool(sequence[i]);
    bools[i] = val;
}
```

The input buffer is overwritten in place with `0`/`1` bytes. This is safe to
model with a separate `Vec<bool>` only because index `i` is **read before it is
written** and indices ascend, so no later read ever sees a rewritten byte — and
because `main` does not touch the buffer again after the call. Had the loop
descended, or had it read `sequence[i+1]`, a separate vector would have
diverged. `lib.rs` documents this at the function and uses `vec![false; len]`.

### 7. Signed/unsigned and width details

- `count`, `len` are `size_t`; `special_count`, `transitions` are `int`.
  Comparisons like `special_count == count` and `transitions == len - 1`
  promote the `int` to `size_t`. Both operands are non-negative wherever these
  run, so the promotion is benign; `lib.rs` casts explicitly and uses
  `count.wrapping_sub(1)` for `count - 1`.
- `configure_flags` caps at 32 decisions (`count = min(length, 32)`), so a
  40-character all-`y` string returns `1032`, not `1040`. Covered by the
  32-element cap cases.
- `1u << i` is only evaluated for `i < 32`, so no undefined shift occurs. The
  resulting `flags` bitmask is computed and **never read** in the C; `lib.rs`
  keeps the computation and discards it with `let _ = flags;`.
- No arithmetic in the translation can overflow an `i32`, so the release build
  (wrapping) and the debug build (overflow panics) behave identically. This
  matters because `cargo test` builds the dev profile.

### 8. Write-failure behaviour

C's `printf`/`fprintf` ignore write errors, whereas Rust's `print!`/`eprint!`
panic on them. Checked with stdout closed (`1>&-`) and stderr closed (`2>&-`):
both programs still exited 0 and 1 respectively, so no divergence is
observable. `main.rs` also ignores the result of its explicit
`stdout().flush()`.

## Branches in the C that no input can reach

Enumerated while looking for untested paths. These are dead in the C, so the
absence of a test for them is a property of the source, not a gap in the suite.
`lib.rs` still mirrors each one structurally.

- `evaluate_conditions`, `logic_op == 2` (XOR), `return 90`: the four preceding
  checks already cover every assignment with an odd number of true conditions.
  Output `90` never appears for operation 1.
- `evaluate_conditions`, `logic_op == 3` (NAND), `return 100`: reaching it needs
  `!(c1 && c2 && c3)` to be true while `c1`, `c2` and `c3` are all true.
- `validate_sequence`, `return 40`: needs `len > 10` with `transitions < 3`, but
  Rule 3 already rejected any run longer than 3, which forces at least
  `ceil(11/3) = 4` runs and hence `transitions >= 3`. Output `40` never appears
  for operation 3.
- `apply_permissions`, the `read && write` fall-through — see section 5.
- `configure_flags`, falling past the `special_count == 1` and
  `special_count == count - 1` search loops: each loop is guaranteed to find its
  element and return.
- `process_decisions`, the `decision_string == NULL` guard: `main` always passes
  a stack buffer.

Reachable-output sets, identical for both programs over the exhaustive sweep:

- operation 0: `-20 -10 -2 -1 0 14 23 35 56 107`
- operation 1: `-2 -1 0 1 2 3 7 10 11 12 50 51 52 100 101 102 103 150 151 152 200`
- operation 2: `-1 0 2..8 100..110 200..210 303..309 502..506 1001..1014` (and
  higher `100+i` / `200+i` / `1000+count` values for longer strings)
- operation 3: `-12 -11 -10 -1 1 2 11 20 25 30 45 50`

## The harness was checked against deliberate regressions

A passing suite proves nothing unless it can fail. Three mutated Rust binaries
were built in a scratch directory and fed to the harness through the
`DRIVER_RUST_BIN` override:

| Mutation | Detected by | Channel that caught it |
|---|---|---|
| `return -2` → `return -3` in `process_decisions` | 8 tests | stdout |
| `ExitCode::from(1)` → `ExitCode::from(0)` on the `fgets` error paths | 3 tests | exit status |
| `"Error reading parameter"` → `"Error reading param"` | 3 tests | stderr |

All three were caught, and each on the channel it corrupted, confirming the
suite is sensitive on all three independently. The mutants were then discarded;
`translation/src` is unchanged from the verified version.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
cd ../../translation && cargo build --release && cargo test
```

`cargo test` prefers `target/release/driver`, falling back to the binary cargo
builds for the test profile. `DRIVER_C_BIN` and `DRIVER_RUST_BIN` override
either path.
