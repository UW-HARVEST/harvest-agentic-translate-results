# Differential verification of the Rust translation

Reference: `c_src/src/main.c`, built with CMake.
Under test: `translation/src/main.rs`, built with `cargo build --release`.
Comparison: both programs run as subprocesses on identical stdin; stdout,
stderr and exit status compared byte for byte.

## Result

**No mismatches were found.** Every input class enumerated below produced
byte-identical stdout, byte-identical stderr (always empty) and exit status 0
from both programs.

Coverage behind that claim:

- `translation/tests/differential.rs` — 23 tests, ~250 distinct inputs, all
  passing under both `cargo test` and `cargo test --release`, none `#[ignore]`d.
- Two ad-hoc differential fuzz runs outside the suite (4,643 and 23,000 inputs)
  over random byte strings and random wide-range numeric literals: 0 mismatches.
  The interesting cases from those runs are folded into the suite, and
  `deterministic_sweep` reproduces a 600-input slice with a fixed seed.

To make sure the harness is capable of failing, it was pointed at a deliberately
wrong program (`RUST_DRIVER_BIN=/bin/cat`): 22 of 23 tests failed, the only
survivor being the `c_src` content-hash guard, which does not run either binary.
A suite that cannot fail proves nothing, so this negative control is part of the
evidence.

## What the C program branches on

The whole program is:

```c
int x = 0;
scanf("%d", &x);
driver(x);          // sets bedrooms = 3, bathrooms = 2.0
                    // then hex-dumps the 16-byte struct image
return 0;
```

`driver` and `print_hex` are unconditional — no `if`, no early `return`, no
length or null check anywhere. The single input-dependent decision is the
outcome of `scanf("%d", &x)`:

| outcome | trigger | effect |
| --- | --- | --- |
| successful conversion | optional whitespace, optional sign, ≥1 digit | `x` = converted value |
| matching failure | first non-whitespace byte cannot start an integer; or EOF right after a sign | `x` keeps `0` |
| input failure | EOF (or a read error) before any non-whitespace byte | `x` keeps `0` |

Because `x` is serialized byte-for-byte into the output, *every distinct
converted value is its own observable case*, which is why the tests sweep bit
patterns and both `int` and `long` boundaries rather than a handful of values.

Baseline output, for reference: `printf("%02x")` over the 16-byte image plus one
`"\n"`. For input `1` that is `01000000030000000000000000000040\n` —
`floors=1` (LE `int`), `bedrooms=3` (LE `int`), `bathrooms=2.0`
(LE IEEE-754 double `0x4000000000000000`). No padding holes on x86-64 LP64, and
`house_t house = {0}` zeroes the object anyway, so no indeterminate bytes are
printed.

## Hazards checked, and why each one is already right

These are the places a naive translation would have diverged. Each was verified
against the C binary rather than reasoned about alone.

1. **`scanf` skips whitespace across newlines.** `%d` consumes a run of
   `isspace()` characters before the number, so `"\n\n\n42"` yields 42, not a
   failure. A `read_line`-based translation would have produced 0 here. The Rust
   code implements the skip explicitly over `' ' \t \n \v \f \r`.
   Test: `scanf_skips_leading_whitespace_across_newlines`, `whitespace_only_input`.

2. **Failed conversion leaves `x` at its previous value.** `scanf` does not zero
   the target on failure; the C code happens to have initialized `x = 0`, so the
   observable value is 0. The Rust `main` only assigns on `Some(..)`, so the
   dependency on the initializer is preserved rather than accidentally
   hard-coded. Test: `matching_failure_leaves_x_zero`, `empty_input`.

3. **Overflow saturates, then truncates.** glibc's `%d` converts with `strtol`
   into a `long`, clamping to `LONG_MAX`/`LONG_MIN`, then stores through an
   `int *`, truncating to 32 bits. The two-stage behavior is visible and
   distinguishes clamping from wrapping:
   - `9223372036854775808` → clamped to `LONG_MAX` → low 32 bits all ones →
     `ffffffff…`
   - `-99999999999999999999999` → clamped to `LONG_MIN` → low 32 bits zero →
     `00000000…`
   - `4294967296` → no clamp, truncates to 0 → `00000000…`

   A translation using `i32::from_str` would have failed the conversion outright
   and printed 0 for the first case. One using wrapping accumulation would have
   printed something else again. Both effects are reproduced in `scanf_d`
   (saturating `i64` accumulation, then `as i32`).
   Tests: `values_beyond_int_are_truncated_like_c`, `long_boundaries_and_overflow`.

4. **Digit runs longer than any buffer.** 100,000 `'9'`s, 100,000 leading `'0'`s
   followed by `5`, and 100,000 spaces before a value all match. The saturating
   accumulator does not care how long the run is, and leading zeros never trip
   saturation. Test: `very_long_digit_runs`.

5. **Conversion stops at the first byte that cannot extend the number, and the
   rest of stdin is never read.** `"0x1f"` is 0 (stops at `x`), `"1.5"` is 1,
   `"1 2"` is 1. Tests: `conversion_stops_at_first_non_digit`,
   `only_first_item_is_read`.

6. **A lone sign, or a sign followed by whitespace or EOF, is a matching
   failure.** `"-"`, `"+"`, `"- 5"`, `"--5"`, `"+-5"` all yield 0.
   Test: `matching_failure_leaves_x_zero`.

7. **Raw bytes.** NUL and `0xff` are neither whitespace nor digits, so they are
   matching failures in leading position and terminators after digits. The Rust
   reader works on bytes, not `char`s, so invalid UTF-8 on stdin cannot make it
   diverge or panic — a `String`-based translation would have. Test:
   `non_ascii_and_nul_bytes`.

8. **Read errors are not input.** With a directory on stdin (`EISDIR`) or
   `/dev/null`, both programs print the `x = 0` image and exit 0; neither writes
   to stderr. Tests: `stdin_is_a_directory`, `stdin_closed`.

9. **Buffering does not change the bytes.** `printf` is line-buffered to a pipe
   and fully buffered to a file; the Rust side uses a `BufWriter` with an
   explicit final flush. Output is identical in both cases, and a dropped flush
   would show up as empty stdout. Test: `stdout_to_a_file_matches`.

10. **`argv` is ignored.** C `main()` takes no parameters, so arguments must not
    change anything. Test: `command_line_arguments_are_ignored`.

11. **Struct layout.** Field order, offsets (`0`, `4`, `8`), little-endian
    encoding and the `2.0` double bit pattern are all exercised: 31 powers of
    two, their negatives, and masks like `0x0102_0304` and `0x00ff_00ff` make
    every byte of the `floors` field non-zero in some case, so a wrong offset or
    a byte-order error could not hide. Tests:
    `every_byte_of_the_value_is_exercised`, `reference_output_shape_is_pinned`.

## Limits of this verification

- Points 3 and 4 depend on glibc's `scanf` implementation. The C standard leaves
  `%d` overflow undefined, so a different libc could legitimately behave
  differently; the translation matches the C program *as built here*, which is
  the stated contract.
- Struct layout assumes the x86-64 System V ABI (LP64, 8-byte-aligned
  `double`). On a target where `sizeof(house_t) != 16` or `int != 4` bytes, the
  hard-coded offsets in `src/main.rs` would need revisiting. Both programs were
  built and compared on x86-64 Linux.
- `c_src/` was not modified. `translation/tests/differential.rs` pins both C
  files by length and FNV-1a hash (`src/main.c`: 1650 bytes,
  `0xe89427e8f472ce6b`; `CMakeLists.txt`: 1200 bytes, `0xcf8f06bd874485bc`) and
  fails if either changes. The test harness configures CMake into
  `translation/target/c_build`, never into the C tree.

## Reproducing

```sh
# reference
cd c_src && cmake -S . -B build && cmake --build build     # -> c_src/build/driver

# translation
cd translation && cargo build --release                    # -> target/release/driver

# differential suite (builds the C reference itself if needed)
cd translation && cargo test --release

# negative control: the suite must fail against a wrong program
cd translation && RUST_DRIVER_BIN=/bin/cat cargo test --release   # expected: 22 failures
```
