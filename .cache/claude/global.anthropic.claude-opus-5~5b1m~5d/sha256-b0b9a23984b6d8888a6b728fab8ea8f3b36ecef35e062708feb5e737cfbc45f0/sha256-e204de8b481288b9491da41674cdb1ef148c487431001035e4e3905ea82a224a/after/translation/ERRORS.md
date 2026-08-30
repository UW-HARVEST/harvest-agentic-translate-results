# Differential verification log — `c_src/src/main.c` vs `translation/`

Method: build both executables, feed identical bytes on stdin, compare
**stdout (byte for byte)**, **stderr (byte for byte)** and **exit status**.
The Rust code is never called as a library; `translation/tests/differential.rs`
spawns both binaries as subprocesses.

* C:    `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver`
* Rust: `cd translation && cargo build --release`
  → `translation/target/release/driver`
* Tests: `cd translation && cargo test`

## Result

**No mismatches remain.** 18 test functions covering ~700 distinct stdin inputs
agree on all three channels. Every input observed produces exit status `0` and
empty stderr in both programs (`main` has a single `return 0`, and neither
program writes to stderr on any path).

## Enumerated branches of the C program and the input class that reaches each

| C location | Condition | Input class | Expected stdout |
|---|---|---|---|
| `main` | `scanf` succeeds, `x != 0` | `1`, `-1`, `42`, `+5`, `007`, `3.7`, `12abc` | `04\ndata value is too large to perform arithmetic safely.\n` |
| `main` | `x == 0` (parsed zero) | `0`, `-0`, `+0`, `000000` | `fffffffe\n` |
| `main` | `scanf` **input failure** (EOF) | empty stdin, whitespace only, closed stdin | `fffffffe\n` (x untouched) |
| `main` | `scanf` **matching failure** | `abc`, `!!!`, `+`, `-`, `- 5`, `.5`, `,`, `0x…`'s leading `0` is a match so see below | `fffffffe\n` |
| `bad` | `data > 0` — always true (`CHAR_MAX`) | any `x == 0` input | `fffffffe\n` |
| `goodG2B` | `data > 0` — always true (`data = 2`) | any `x != 0` input | `04\n` |
| `goodB2G` | `data > 0` true, `data < CHAR_MAX/2` **false** | any `x != 0` input | the `printLine` message |
| `goodB2G` | `data < CHAR_MAX/2` true branch | **unreachable**: `data` is unconditionally `CHAR_MAX` (127), and `127 < 63` is false | — |
| `printLine` | `line != NULL` | the only call site passes a literal | message printed |
| `printLine` | `line == NULL` | **unreachable**: no call site passes NULL | — |

Note the two dead paths are genuinely unreachable in the C, so no input class
exists for them; the Rust mirrors the same structure rather than deleting them.

## C behaviours that had to be replicated exactly (verified, not assumed)

These are the places a naive translation diverges. Each was confirmed against
the compiled C binary rather than reasoned about on paper.

1. **Signed-char overflow in `bad()` printed as an unsigned int.**
   `data = CHAR_MAX` (127); `char result = data * 2` computes `254` in `int`
   then truncates to `char`, which is **signed** on x86-64 Linux → `-2`.
   `printf("%02x", result)` promotes `-2` to `int` and `%x` reinterprets it as
   `unsigned int`, so the output is `fffffffe`, **not** `fe` and not `-2`.
   The `02` width is therefore inert here. Rust: `{:02x}` applied to
   `(char_hex as i32) as u32`. Pinned as a literal expectation in
   `zero_takes_the_bad_branch`.

2. **`CHAR_MAX/2` is integer division** → `63`, so the `else` arm of `goodB2G`
   always runs. Rust uses `i8` division, giving the same `63`.

3. **`scanf("%d")` reads across newlines**, unlike `fgets`. It skips *all*
   leading whitespace (space, `\t`, `\n`, `\v`, `\f`, `\r`) before the number,
   so `"\n\n  \n  42"` yields `42`. The reader in `main.rs` implements the same
   skip set (`is_c_space`) and consumes exactly the bytes `%d` would, with one
   byte of push-back.

4. **On `scanf` failure `x` keeps its initialiser `0`**, so the program silently
   takes the `bad()` branch instead of erroring. Both matching failure (`"abc"`)
   and input failure (empty stdin) behave this way, and the return value of
   `scanf` is never checked. There is **no error message and no non-zero exit**
   on any input — a translation that printed a parse error or exited `1` would
   diverge on every malformed input.

5. **glibc parses `%d` via `strtol`, which saturates, and the result is then
   stored through an `int *` — so it truncates.** This is the subtlest case and
   it *flips the branch*:
   * `"99999999999999999999"` → saturates to `LONG_MAX` → `(int)LONG_MAX == -1`
     → non-zero → `good()`.
   * `"-99999999999999999999"` → saturates to `LONG_MIN` → `(int)LONG_MIN == 0`
     → **zero** → `bad()`.
   * `"4294967296"` (2^32) → truncates to `0` → `bad()`, even though the input
     is not zero.
   Rust reproduces this with an `i64` accumulator that saturates to
   `i64::MAX`/`i64::MIN` on overflow, then `as i32`. Covered by
   `long_boundaries_and_saturation` and
   `values_that_truncate_to_zero_flip_the_branch`.

6. **Only the first conversion is consumed.** `"1 2 3"` reads `1`; `"0abc"`
   reads `0`; `"3.7"` reads `3` and leaves `.7`; `"0x10"` reads `0` and stops at
   `x`, so it takes the `bad()` branch. Trailing input is never read.

7. **A lone sign is a matching failure**, not zero: `"+"`, `"-"` and `"- 5"`
   all leave `x == 0`. (`"- 5"` fails because `%d` does not allow whitespace
   between the sign and the digits.)

8. **`argv` is ignored** — `main` is declared `int main()` and takes no
   parameters, so arguments must not change behaviour or produce usage output.

9. **Trailing newlines in output.** Both `printLine` and `printHexCharLine` use
   an explicit `\n` in the format string, so every line ends with exactly one
   newline and there is no extra trailing blank line.

## Test inventory (`translation/tests/differential.rs`)

`both_binaries_are_runnable`, `zero_takes_the_bad_branch`,
`nonzero_takes_the_good_branch`, `negative_is_nonzero_and_takes_good`,
`empty_input_leaves_x_untouched`, `whitespace_only_inputs`,
`matching_failure_non_numeric`, `scanf_reads_across_newlines`,
`only_the_first_conversion_is_consumed`, `int_boundaries`,
`values_that_truncate_to_zero_flip_the_branch`,
`long_boundaries_and_saturation`, `signed_zero_and_leading_zeros`,
`very_long_digit_runs` (up to 100 000 digits),
`binary_and_non_ascii_stdin` (NUL bytes, `0x00`–`0xff`, UTF-8),
`command_line_arguments_are_ignored`, `sweep_small_integers` (−300…300),
`sweep_char_boundary_values`.

No test is `#[ignore]`d, skipped or disabled.
