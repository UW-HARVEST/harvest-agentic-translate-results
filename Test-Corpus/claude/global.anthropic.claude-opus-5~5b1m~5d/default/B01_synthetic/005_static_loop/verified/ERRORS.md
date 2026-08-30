# Differential verification log — `driver` (C → Rust)

Ground truth: `c_src/src/main.c`. The Rust binary must match it byte-for-byte on
stdout and stderr, and match its exit status, for every input.

## How each program is run

| | command |
|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` → `c_src/build/driver ARGS...` |
| Rust | `cd translation && cargo build --release` → `translation/target/release/driver ARGS...` |

Both build with no errors and no warnings. Tests: `cd translation && cargo test`
(23 tests, 0 failed, 0 ignored). The tests spawn **both binaries as
subprocesses** and compare stdout / stderr / exit status; the Rust code is never
loaded as a library.

## Result summary

**No behavioral mismatches were found.** The translation in `src/main.rs` already
reproduced every branch of the C program correctly. Every input class enumerated
below produced identical stdout, identical stderr and an identical exit status.

Beyond the 23 named tests, an independent sweep of **~6,800 additional argv
values** (random junk strings over `0-9 + - space tab a b x X . \n \r`, random
integers spanning ±2^36, and all 1- and 2-byte strings over an interesting
alphabet) produced **0 mismatches**.

Because nothing was found to fix, the sections below record (a) the C behaviors
that were the likely places for a mismatch and were explicitly confirmed, and
(b) the mutation testing used to prove the suite is not passing vacuously.

## Branches in the C source and the input class that reaches each

`main.c` has exactly three exit paths and one loop:

| C line | Branch | Reaching input | Observed (both programs) |
|---|---|---|---|
| `if (argc != 2)` | too few args | *(no arguments)* | `Error: should only be a single (integer) argument!\n` on **stdout**, exit **1** |
| `if (argc != 2)` | too many args | `1 2`, `1 2 3` | same message, exit **1** |
| `if (end == argv[1])` | strtol converted nothing | `""`, `abc`, `-`, `+`, `--5`, `.5`, `e5`, whitespace-only, `" -"` | `Error: first argument must be an integer!\n` on **stdout**, exit **1** |
| `for (i=0..9)` | happy path | any arg strtol advances past | 10 newline-terminated lines, exit **0** |

## Subtle C behaviors explicitly confirmed to match

1. **Diagnostics go to stdout, not stderr.** Both error messages use `printf`,
   so they land on **stdout** and stderr stays **empty**. A translation that
   used `eprintln!` would still pass a stdout-only test on the happy path;
   `argc_error_message_goes_to_stdout_not_stderr` and `parse_error_message_and_status`
   pin this down.

2. **Order of checks.** `argc != 2` is tested *before* the argument is parsed, so
   `driver a b` reports the *argument-count* error, not the parse error.

3. **`strtol` accepts trailing garbage.** The guard is `end == argv[1]`, i.e.
   "nothing at all was parsed" — *not* "the whole string was parsed". So
   `5abc` → stride 5, `0x10` → stride 0, `1e5` → 1, `12.75` → 12, `-12xyz` → -12
   all **succeed** with exit 0. Covered by
   `trailing_garbage_is_accepted_because_end_advanced`.

4. **`strtol` skips leading whitespace and honours a sign.** `"  5"`, `"\t5"`,
   `" \t\n\v\f\r-8"`, `"+42"` all parse. `isspace` set replicated as
   `' ' \t \n \v \f \r` in `c_isspace`.

5. **`strtol` saturates, then the result is truncated to `int`.** This is the
   two-step behavior most likely to be mistranslated:
   `strtol` returns `long` clamped to `LONG_MAX`/`LONG_MIN` on overflow, and
   `int stride = strtol(...)` then **truncates** that 64-bit value to 32 bits.
   - `99999999999999999999` → `LONG_MAX` (`0x7FFF…FF`) → truncated to `int` = **-1**
   - `-99999999999999999999` → `LONG_MIN` → truncated to `int` = **0**
   - `2147483648` → fits a `long`, truncates to **-2147483648**
   - `4294967296` → truncates to **0**
   Covered by `strtol_saturates_then_truncates` and `long_to_int_truncation`.
   Note this means saturation is *not* equivalent to clamping into `int` range —
   see mutant 3 below, which the suite catches.

6. **Signed overflow in the loop wraps (as GCC/Clang emit it).** `i * stride`
   and the accumulating `static int sum` both overflow `int` for large strides.
   The C is technically UB here, but the compiled program wraps two's-complement,
   and the Rust matches using `wrapping_mul` / `wrapping_add`. Verified for
   `2000000000` (output `0 2000000000 1705032704 -884901888 …`), `2147483647`,
   `-2147483648`, and the values straddling the boundary `238609294`/`238609295`
   (`238609294 * 45` is right at `INT_MAX`). Covered by
   `multiplication_and_sum_overflow`.

7. **`static int sum` is function-local persistent state.** Modeled with a
   `thread_local! { Cell<i32> }`. Behaviorally identical here because the
   program is single-threaded and the process is short-lived.

8. **Non-UTF-8 argv is passed through byte-for-byte.** C sees raw bytes, so the
   Rust reads the argument via `OsStr::as_bytes()` rather than `String`, and
   parses bytes directly. `\xff\xfe` → parse error; `5\xff` → stride 5.
   A `String::from_utf8`-based translation would diverge (or panic) here.
   Covered by `non_utf8_arguments`.

9. **Exactly 10 lines, each `%d\n`, no trailing blank line.** Pinned by
   `happy_path_prints_ten_newline_terminated_lines`, which asserts the literal
   bytes `0\n3\n9\n18\n30\n45\n63\n84\n108\n135\n` for stride 3.

## Paths that are unreachable rather than untested

- **`argc == 0`.** Reachable only via `execve` with an empty `argv`. Attempted
  directly with `fork`/`execv`; the spawn itself fails identically for both
  binaries, so this path cannot be differentiated and is not a mismatch. Had it
  been reachable, both take the `argc != 2` branch anyway.
- **Write failures on stdout** (`>&-`, `>/dev/full`). Both programs ignore
  `printf`/`write` errors and still exit **0**; confirmed identical. The Rust
  deliberately discards the `Result` of each write (`let _ = …`) to match.

## Mutation testing — proof the suite is not vacuous

Five deliberate defects were injected into `src/main.rs`, one at a time, and the
suite was re-run. Four were caught immediately; the fifth was caught once the
injection was applied correctly (the first attempt's regex silently failed to
match, which is why it initially reported green):

| Injected defect | Suite result |
|---|---|
| `strtol` overflow returns 0 instead of saturating to `LONG_MAX` | **FAILED** (1 test) |
| Clamp into `int` range instead of truncating `long`→`int` | **FAILED** (3 tests) |
| Loop runs 9 times instead of 10 | **FAILED** (15 tests) |
| Error text reworded (`integer` → `INTEGER`) | **FAILED** (7 tests) |
| Error paths exit 0 instead of 1 (stdout unchanged) | **FAILED** (11 tests) |

The last row is the important one: it changes **only the exit status**, leaving
stdout byte-identical. A stdout-only comparison would have passed it. All
mutations were reverted and the suite is green (23/23) against the restored
source; `c_src/` was never modified.
