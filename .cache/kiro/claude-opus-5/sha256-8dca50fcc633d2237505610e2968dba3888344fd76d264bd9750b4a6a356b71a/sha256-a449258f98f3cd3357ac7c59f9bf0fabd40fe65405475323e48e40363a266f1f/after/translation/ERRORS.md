# Differential verification log

Reference: `c_src/src/main.c` (built with cmake as `c_src/build/driver`).
Under test: `translation/src/main.rs` (built as `translation/target/{debug,release}/driver`).

Run commands recorded in Phase A:

```
c_src/build/driver                  # C reference
translation/target/release/driver   # Rust translation
```

## Mismatches found

**None.** Every input class enumerated below produced byte-identical stdout,
byte-identical stderr and the same exit status. The Rust source already handled
each of the C program's quirks, so no code changes to `translation/src/` were
required and no test was weakened, disabled or `#[ignore]`d.

Verification was not limited to the assertions in `tests/differential.rs`: an
independent exhaustive shell sweep ran both binaries over all 256 possible
first bytes plus the empty-input and read-error cases and diffed the three
observable channels. Zero divergences.

## Traps that were checked, and why each could have produced a mismatch

These are the places a naive translation *would* have diverged. Each is listed
with the C behavior, the failure a wrong translation shows, and the test that
pins it.

### 1. `fscanf` failure leaves `data` at its initializer

```c
char data;
data = ' ';
fscanf(stdin, "%c", &data);
```

On empty stdin, closed stdin, or a read error, `%c` performs **no assignment**.
`data` therefore stays `' '` (0x20) and the program prints `21`, not `01` and
not an error.

A translation that read into a zero-initialized buffer and used it
unconditionally prints `01` on empty input. A translation that treated EOF as
fatal exits non-zero.

Tests: `empty_stdin`, `stdin_closed`, `stdin_is_a_directory`.

### 2. `%c` does not skip leading whitespace

Unlike `%d` or `%s`, the `%c` conversion has no leading-whitespace skip. A
newline, space or tab arriving first is consumed **as the data byte**.

A translation that trimmed input or read a "line" and skipped blanks reports
`42` for `"\nA"` where C reports `0b`.

Tests: `leading_whitespace_is_consumed_as_data`, `single_space_matches_eof_output`.

Note the coincidence that makes this easy to miss: a single space produces `21`,
the same output as the EOF path. `single_space_matches_eof_output` records that
the two agree by accident, not by design.

### 3. `char` is signed, and the variadic promotion sign-extends

```c
char result = data + 1;
printHexCharLine(result);   // void printHexCharLine(char charHex)
printf("%02x\n", charHex);  // charHex promoted char -> int, printed as unsigned
```

`data + 1` is computed in `int` (integer promotion), then truncated back to
`char`. On x86-64 Linux `char` is signed, so any result above 0x7f becomes
negative. Passing that `char` through `...` promotes it back to `int` with sign
extension, and `%x` reinterprets the `int` as `unsigned int`.

Consequence: the field is *eight* hex digits wide for negative results.

| stdin byte | C `result` | printed  |
|-----------:|-----------:|:---------|
| `0x7e`     | `0x7f`     | `7f`     |
| `0x7f`     | `-128`     | `ffffff80` |
| `0x80`     | `-127`     | `ffffff81` |
| `0xfe`     | `-1`       | `ffffffff` |
| `0xff`     | `0`        | `00`      |

A translation using `u8` arithmetic prints `80`, `81`, `ff`, `00` — matching
only on the last row. A translation using `i32` without truncating to `i8`
prints `80` for input `0x7f`.

Tests: `signed_char_boundaries`, `all_256_first_bytes`, `utf8_multibyte_first_byte_only`.

### 4. `%02x` is a minimum width, not a truncation

Zero padding applies only when the value is narrower than two digits (`0x00` ->
`01`). It never clips the eight-digit sign-extended values from trap 3.

Tests: `zero_padding_width`.

### 5. NUL is an ordinary byte

`%c` reads one byte with no string semantics, so `"\0abc"` yields `01`. A
translation that treated stdin as a C string or a `&str` would stop early or
fail UTF-8 validation.

Tests: `nul_byte`, `large_nul_input`, `full_byte_range_streams`.

### 6. Non-UTF-8 input must not fail

Bytes `0x80`–`0xff` are not valid standalone UTF-8. A translation that read
stdin via `read_to_string` or `lines()` errors out or panics where C prints a
value.

Tests: `all_256_first_bytes`, `utf8_multibyte_first_byte_only`, `full_byte_range_streams`.

### 7. Exactly one byte is consumed; the rest is ignored

The program exits after the single conversion. The writer on the other end of
the pipe may get `EPIPE`; that must not appear on either program's stderr or
change the exit status.

Tests: `only_first_byte_is_read`, `all_256_first_bytes_with_trailer`,
`very_large_input` (1 MiB), `large_nul_input` (64 KiB).

### 8. stdout buffering must not change the byte stream

C's stdout is line buffered to a terminal and fully buffered to a file or pipe;
either way `exit` flushes it. The Rust translation flushes explicitly at the end
of `main`. Checked against both a pipe and a regular file.

Tests: `stdout_to_regular_file`, plus every piped test.

### 9. `main()` takes no arguments

`argv` is never inspected, so extra command-line arguments change nothing.

Tests: `arguments_are_ignored`.

### 10. Exit status is always 0

`return 0` is the only exit from `main`; there is no error path that changes it.
A test asserting stdout alone would pass while the Rust program exited non-zero,
so all three channels are asserted on every comparison.

Tests: `exit_status_always_zero`, and the three-channel assertion in `assert_same`.

## Coverage against the C control flow

`main.c` has no `if`, no loop and no early `return`, so branch coverage reduces
to the input classes of the two library calls. All are covered:

- `fscanf` returns 1 (byte available) — all 256 values, exhaustively
- `fscanf` returns `EOF` at end of input — `empty_stdin`
- `fscanf` returns `EOF` on a read error — `stdin_is_a_directory` (EISDIR),
  `stdin_closed`
- `printf` with a value needing zero padding, needing none, and exceeding the
  field width — `zero_padding_width`

## Completion gate (Phase D)

- both programs build with no errors — yes
- every enumerated input gives identical stdout, stderr and exit status — yes
- `cargo test` passes in `translation/` — yes, 19/19 in both debug and release
- no test disabled, skipped or `#[ignore]`d — none
- `c_src/` unmodified — confirmed; only `c_src/build/` (cmake output, untracked)
  was created
