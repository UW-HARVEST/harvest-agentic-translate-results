# Verification log: C → Rust differential testing

Reference: `c_src/src/main.c` (built with CMake as `c_src/build/driver`).
Translation: `translation/src/main.rs` (built as `translation/target/release/driver`).

## How the two programs are run

```
# C reference
cd c_src && cmake -S . -B build && cmake --build build     # -> c_src/build/driver
c_src/build/driver  < input

# Rust translation
cd translation && cargo build --release                     # -> target/release/driver
translation/target/release/driver < input
```

`translation/tests/differential.rs` spawns both binaries as subprocesses, pipes
the same bytes to stdin, and asserts stdout, stderr and exit status all match.
The C binary is built automatically by the test harness if it is missing, so
`cargo test` in `translation/` is self-contained. No test loads the Rust code as
a library and no test is `#[ignore]`d.

## Input space

The C program's entire observable input is *the first byte of stdin, or the
absence of one*. `fscanf(stdin, "%c", &data)` consumes exactly one byte and
nothing else is ever read, so the input alphabet is finite and is covered
**exhaustively**: all 256 possible first bytes, plus the EOF case. Trailing
input, argv, input size and stdin source are covered as separate classes to
confirm they are genuinely ignored.

## Branch / input-class enumeration

| Class | Input | C behaviour | Test |
|---|---|---|---|
| EOF (empty stdin) | `""` | `fscanf` returns `EOF`, return value ignored, `data` keeps its pre-initialised `' '`; prints `21` | `empty_stdin`, `empty_stdin_matches_preinitialised_space_plus_one` |
| EOF via `/dev/null` | closed/empty stdin | same as above | `stdin_from_null_device` |
| Ordinary byte | `0x21`–`0x7e` | `data + 1`, two hex digits | `single_byte_printable` |
| Every first byte | `0x00`–`0xff` | exhaustive | `every_possible_first_byte` |
| Leading whitespace | `\n`, ` `, `\t`, `\r`, `\v`, `\f` | `%c` does **not** skip whitespace — consumed as data | `whitespace_is_data_not_skipped` |
| Signed-`char` wrap | `0x7f`, `0x80`, `0xfe`, `0xff` | wraps in `char`, then sign-extends in `printf` | `signed_char_overflow_and_sign_extension`, `sign_extension_golden_values` |
| NUL bytes | `\0…` | ordinary data | `nul_bytes` |
| Trailing input | `"AB"`, `"a\nb\nc\n"`, … | only the first byte is read | `only_first_byte_is_consumed` |
| Non-UTF-8 | `\xff\xfe`, `\xc3`, `\xed\xa0\x80` | raw bytes, no decoding | `invalid_utf8_input` |
| Oversized input | 1 MiB | unread remainder; must not block | `very_large_input` |
| argv present | `--help -x garbage` | `int main()` ignores argv | `arguments_are_ignored` |
| stderr / status | all of the above | stderr always empty, always exits `0` | `stderr_always_empty`, `exit_status_always_zero` |

## Mismatches found

**None.** Across all 264 enumerated inputs (256 single bytes + EOF + the
multi-byte, oversized, argv and `/dev/null` classes), stdout, stderr and exit
status were byte-identical on the first run. The translation as delivered already
reproduced every quirk below; no change to `translation/src/main.rs` was needed.

## Quirks that had to be reproduced, and how the Rust does it

These are the mismatches this suite is designed to catch. Each was verified to be
*load-bearing* by deliberately breaking the Rust, watching the suite fail, and
reverting (see "Negative control" below).

1. **Ignored `fscanf` return value + pre-initialised `data`.**
   `data = ' '` runs before the read, and the C never checks whether `fscanf`
   succeeded. On empty stdin `data` therefore stays `0x20` and the program
   prints `21`, not an error and not `00`. It exits `0` — there is no error
   path that changes the status.
   Rust: `scanf_one_char()` returns `None` on EOF/error and the caller simply
   leaves `data` at `b' ' as i8`.

2. **`%c` does not skip leading whitespace.**
   Unlike `%d`/`%s`, the `%c` conversion performs no whitespace skipping, so a
   leading `\n` is the datum. Input `"\nX"` prints `0b` (`'\n' + 1`), never
   `59` (`'X' + 1`).
   Rust: a raw one-byte `Read::read`, with no trimming and no line-based input.

3. **`char` is signed, and `char result = data + 1` truncates back to `char`.**
   `data + 1` is computed in `int` but stored back into a `char`, so `0x7f`
   becomes `-128` and `0xff` becomes `0`.
   Rust: `data.wrapping_add(1)` on an `i8`.

4. **`printf("%02x", charHex)` sign-extends negative values to 8 hex digits.**
   The `char` argument is default-promoted to `int`; `%x` then reinterprets that
   `int` as `unsigned int`. For negative values this yields the sign-extended
   32-bit pattern, whose 8 digits make the `02` minimum width irrelevant. This
   is the highest-value trap in the program — a translation that printed the
   byte as unsigned would produce `80` where C produces `ffffff80`, matching on
   129 of 256 inputs and silently differing on the rest.
   Rust: `char_hex as i32 as u32` formatted with `{:02x}`.
   Golden values pinned in `sign_extension_golden_values`:
   | first byte | C stdout |
   |---|---|
   | (EOF) | `21\n` |
   | `0x00` | `01\n` |
   | `0x1f` | `20\n` |
   | `0x7e` | `7f\n` |
   | `0x7f` | `ffffff80\n` |
   | `0xfe` | `ffffffff\n` |
   | `0xff` | `00\n` |

5. **Trailing `\n` and no other output.** `printHexCharLine` emits exactly one
   `\n` after the digits, nothing to stderr, and `main` returns `0`
   unconditionally. Rust matches, and explicitly flushes stdout before exit
   (Rust's line-buffered stdout is flushed at drop, but the explicit flush makes
   the ordering independent of whether stdout is a pipe or a TTY).

## Negative control

To prove the suite is not vacuous, `print_hex_char_line` was temporarily changed
from `char_hex as i32 as u32` to `char_hex as u8 as u32` (i.e. dropping quirk 4).
4 of 15 tests failed — `every_possible_first_byte`, `invalid_utf8_input`,
`signed_char_overflow_and_sign_extension` and `sign_extension_golden_values` —
first reported at input `0x7f`. The change was then reverted and the suite is
green again. The harness additionally asserts the C reference produced non-empty
stdout and exited `0`, so a comparison in which neither program actually ran
cannot pass.

## Final state

- `c_src/` unmodified (`c_src/src/main.c` md5 `84a0c7592685d031a43b86ce86b7148a`);
  only the untracked CMake output directory `c_src/build/` was added.
- `cargo build --release`: clean, no errors or warnings.
- `cargo test` and `cargo test --release`: 15 passed, 0 failed, 0 ignored.
