# Differential verification log

Reference: `c_src/src/main.c` (never modified). Translation: `translation/src/main.rs`.

Both programs are compared by execution only: `tests/differential.rs` spawns each
binary with identical `argv` and requires byte-identical stdout, byte-identical
stderr, and an identical wait status. Nothing is loaded as a library.

Commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> translation/target/release/driver
cd translation && cargo test
```

## Mismatches found

### 1. Exit status differed when stdout was closed early (SIGPIPE)

**Symptom.** With a subject long enough to fill the pipe buffer, and the reader
gone:

```
$ c_src/build/driver "$(python3 -c "print('x'*100000)")" | head -c 10 >/dev/null
status 141          # killed by signal 13
$ translation/target/release/driver "$(...)" | head -c 10 >/dev/null
status 0            # exited normally
```

stdout and stderr agreed; only the status differed, which is exactly the failure
mode a stdout-only assertion misses.

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs. The failing `write` therefore returned `EPIPE` instead of terminating the
process, and `run()` discards write errors with `let _ = ...` (which is itself
correct — see "Deliberately preserved behaviour" below), so the program fell
through to `ExitCode::from(0)`. The C program keeps the default disposition and
is killed by signal 13 mid-`printf`.

**Fix.** `restore_default_sigpipe()` in `src/main.rs` resets `SIGPIPE` to
`SIG_DFL` as the first statement of `main`. Covered by
`stdout_closed_early_kills_both_programs_the_same_way`.

**Confirmed no other mismatch.** Every other input class below already agreed.
Across a 7718-case shell/Python sweep and the 22 tests in
`tests/differential.rs`, this was the only divergence observed.

## Branch enumeration

Each `if` and early `return` in `main.c`, the inputs that reach it, and the test
that covers it.

| # | C branch | Reaching input | Result | Test |
|---|---|---|---|---|
| 1 | `argc == 1` | no arguments | usage, exit 1 | `argc_one_prints_usage` |
| 2 | `argc > 4` | 4+ operands | usage, exit 1 | `argc_above_four_prints_usage` |
| 3 | `argc == 2` | subject only | whole string | `single_argument_prints_whole_string`, `..._passes_bytes_through_unchanged` |
| 4 | `end == argv[2]` | `""`, `"abc"`, `" "`, `"+"`, `"-"`, `"\xff"`, … | `Second argument must be an integer!`, **no newline**, exit 1 | `second_argument_not_an_integer` |
| 5 | `start > len` | `6`, `100`, `-1`, `2147483648`, `9223372036854775807` | `start is off the end`, exit 1 | `start_off_the_end_of_the_string` |
| 6 | neither | `0`, `len-1`, `len` | substring (empty at `start == len`) | `start_at_the_boundaries_is_accepted`, `start_uses_strtol_parsing_rules` |
| 7 | `end == argv[3]` | **unreachable** | — | `third_argument_error_message_is_unreachable` |
| 8 | `stop > len` | `6`, `-1`, `2147483648`, `9223372036854775807` | `stop is off the end`, exit 1 | `stop_off_the_end_of_the_string` |
| 9 | `stop <= start` | `2 2`, `3 2`, `0 0`, and any unparsable third arg | `stop must come after start!`, exit 1 | `stop_must_come_after_start`, `non_integer_third_argument_falls_through_to_the_ordering_check` |
| 10 | `else stop = len` | `argc == 3` | stop defaults to end | `start_at_the_boundaries_is_accepted` |
| 11 | final `printf` | valid `[start, stop)` | substring + `\n` | `valid_ranges_print_the_substring`, `every_valid_range_of_a_short_string` |

`every_valid_range_of_a_short_string` sweeps `start` and `stop` over
`-2 ..= len+2` for subjects of length 0, 1, 2 and 8, so rows 5, 6, 8, 9 and 11
are each hit many times, including at every boundary.

## Deliberately preserved behaviour

These look like defects. The C is the ground truth, so they are reproduced, not
repaired.

- **`Second argument must be an integer!` has no trailing newline**, unlike every
  other message. `$(...)` in a shell strips trailing newlines, so a naive harness
  cannot see this; the tests compare raw bytes.
- **`Third argument must be an integer!` is dead code.** `strtol(argv[3], NULL, 10)`
  passes `NULL` for `endptr`, so `end` still holds the pointer from the `argv[2]`
  conversion. That pointer always lies within `[argv[2], argv[2]+strlen(argv[2])]`,
  and `argv[3] == argv[2] + strlen(argv[2]) + 1`, so `end == argv[3]` is off by at
  least one and can never hold. Consequence: an unparsable third argument yields
  `stop == 0` and is reported as `stop must come after start!` instead.
  `third_argument_error_message_is_unreachable` sweeps 4x18x18 = 1296 inputs and
  asserts the C program never emits the message.
- **`start > len` / `stop > len` compare `int` against `size_t`.** The usual
  arithmetic conversions widen the signed `int` to `unsigned long`, so `-1`
  becomes `ULONG_MAX` and negative indices are reported as "off the end" rather
  than as invalid. Rust reproduces this with `(start as u64) > len`, relying on
  `i32 as u64` sign-extending.
- **`long` -> `int` truncation.** `strtol` returns `long`; `start`/`stop` are
  `int`. So `4294967298` acts as `2`, `4294967296` acts as `0`, and
  `2147483648` acts as `INT_MIN` (rejected as off the end). Rust uses `value as i32`.
- **`strtol` saturation.** Out-of-range input yields `LONG_MAX`/`LONG_MIN`.
  `(int)LONG_MAX == -1` (off the end), while `(int)LONG_MIN == 0`, so
  `-9223372036854775808` is silently accepted as index 0. The hand-written
  `strtol` accumulates in the negative direction so `LONG_MIN` is representable,
  and flags the `+2^63` case separately.
- **Write errors are ignored.** The C code never checks `printf`'s return value,
  so a full or unwritable stdout still exits 0. Verified against `/dev/full` by
  `unwritable_stdout_is_ignored_identically`. This is why `run()` uses
  `let _ = out.write_all(..)`; the SIGPIPE fix above addresses the status
  difference without introducing error checking the C does not perform.
- **Bytes are passed through, not text.** `argv` may hold non-UTF-8. The Rust
  code reads `args_os()` and takes raw bytes via `OsStrExt`, never
  `to_string_lossy`, so `\xff\xfe\xfd` round-trips.

## Known-unreachable path (not a mismatch)

`else stop = len` truncates `size_t` to `int`. This would differ from a clamping
implementation only when `strlen(argv[1]) > INT_MAX`. Linux caps a single `argv`
entry at `MAX_ARG_STRLEN` (32 pages); measured on this host, 131071 bytes
succeed and 131072 fails with `E2BIG`. `len` therefore cannot approach
`INT_MAX`, and the branch is unreachable through the process interface. Recorded
because mutation testing showed it is the one change to `src/main.rs` the suite
cannot detect — by construction, not by omission.

## Verification performed

- Both binaries build with no errors; `cargo build --release` and
  `cargo build` are warning-free.
- `cargo test`: 22 passed, 0 failed, **0 ignored**. `cargo test --release`
  likewise.
- Ad-hoc sweeps outside the test suite: 207 shell cases and 7718 Python cases
  (exhaustive small grids plus 6000 randomized inputs), all byte-identical.
- Mutation testing, to show the suite is not vacuous. Each mutation was applied
  to `src/main.rs`, `cargo test` was run, and the source was restored:

  | Mutation | Detected by |
  |---|---|
  | newline appended to the second-argument message | 6 tests |
  | `start > len` compared as signed | 6 tests |
  | `SIGPIPE` restore removed | `stdout_closed_early_...` |
  | `stop <= start` weakened to `stop < start` | 6 tests |
  | `stop = len as i32` clamped instead of truncated | none — unreachable, see above |

- `c_src/` sources untouched: `src/main.c` and `CMakeLists.txt` retain their
  checkout mtimes. The only addition under `c_src/` is the `build/` directory
  produced by CMake.
