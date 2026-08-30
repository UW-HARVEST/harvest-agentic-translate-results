# Differential verification: `c_src/src/main.c` vs `translation/src/main.rs`

## Result

**No behavioral mismatches were found.** Across ~11,300 distinct inputs the two
binaries produced byte-identical stdout, byte-identical stderr (always empty)
and the same exit status (always 0). No change to `translation/src/main.rs` was
required, and nothing in `c_src/` was modified.

Because "I found nothing" is only useful if you can see where I looked, the rest
of this file records every place the translation *could* have diverged, what the
C actually does there, and how the Rust matches it.

## How it was verified

- C: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver`
- Rust: `cd translation && cargo build --release`
  → `translation/target/release/driver`
- Differential suite: `translation/tests/differential.rs` (17 tests). It builds
  the C binary via CMake if missing, then spawns **both executables as
  subprocesses**, pipes the same bytes to stdin, and compares stdout, stderr and
  exit status. The Rust code is never loaded as a library.
- Both profiles pass: `cargo test` and `cargo test --release`.
- The C was also rebuilt with *no* `CMAKE_BUILD_TYPE` (the command in the task,
  i.e. no optimization flags) and with `-DCMAKE_BUILD_TYPE=Release` (`-O3`).
  Results were identical under both, which matters because `bedrooms +=` is
  signed overflow — see below.

## Branch inventory of the C program

Every branch point in `main.c`, and the test that reaches each side:

| Location | Branch | Reached by |
|---|---|---|
| `main` | `fgets` returns `NULL` (immediate EOF) | `empty_input_reaches_the_error_path`, `stdin_dev_null` |
| `main` | `fgets` returns `NULL` (read error, `EISDIR`) | `stdin_read_error` |
| `main` | `fgets` succeeds | all success-path tests |
| `main` | `parse_val` → `true` | `single_item`, `successful_conversions_with_strtol_quirks` |
| `main` | `parse_val` → `false` (`"An error occurred\n"`) | `no_conversion_performed` |
| `parse_val` | `endp == str` (no conversion) | `no_conversion_performed` |
| `parse_val` | `errno != 0` (`ERANGE`) | `long_range_and_erange` |
| `parse_val` | `tmp < INT_MIN` | `int_range_boundaries` |
| `parse_val` | `tmp > INT_MAX` | `int_range_boundaries` |
| `parse_val` | all three conditions hold | success-path tests |

`run`, `print_house`, `add_floor` and `add_bedrooms` are branch-free.

## Divergence risks examined, and why they do not occur

### 1. `fgets` does not read across newlines

`char in[100]` with `fgets(in, sizeof(in), stdin)` reads **at most 99 bytes**,
stops after the first `\n` (which it keeps), and never consumes a later line.
The Rust does `stdin.lock().take(99).read_until(b'\n', &mut buf)`, which has the
same three stopping conditions.

The 99-byte boundary is where an off-by-one would hide, so it is tested from
both sides: 98 chars + `\n` (exactly 99 bytes, newline included), 99 chars +
`\n` (newline *not* read), 100 digits, `"1" * 120` (truncated to 99 digits,
which then becomes `ERANGE` → error path), `"1"` followed by 105 `'0'`s, a `'-'`
sitting on the 99th byte, `INT_MAX` ending exactly at the boundary, and 200
spaces before the digits (the digits fall outside the window, so `strtol`
performs no conversion → error path).
See `fgets_99_byte_boundary`, `fgets_does_not_read_across_newlines`.

### 2. `fgets` failure leaves the buffer as `""`, not indeterminate

`in` is initialized with `= ""`, so it is 100 zero bytes. When glibc `fgets`
returns `NULL` it does not write to the buffer, so `in` is still the empty
string; `strtol("")` performs no conversion, `endp == str`, and the program
prints `An error occurred` and **still returns 0**. The Rust ignores the
`read_until` error and leaves `buf` empty, producing the same output and the
same exit status. Verified with `/dev/null` (EOF) and with a directory on stdin
(EISDIR).

### 3. Embedded NUL bytes truncate the string

`fgets` copies a NUL from the stream like any other byte, but `strtol` then
treats it as the end of the string. The Rust truncates the buffer at the first
NUL after reading. Tested with a leading NUL (→ error path), a NUL between
digits, a NUL after a complete number, and a NUL after the sign.
See `embedded_nul_bytes_terminate_the_c_string`.

### 4. `strtol` accepts leading whitespace, a sign, and trailing garbage

`parse_val` checks only `endp != str`, so `"12abc"`, `"0x10"` (base 10 stops at
`x`), `"1e5"`, `"4.9"`, `"5\r"` from a CRLF line, and `"  42"` are all
**accepted**, yielding 12, 0, 1, 4, 5, 42. Conversely `"+"`, `"-"`, `"--5"`,
`" - 5"`, `".5"` and `"_5"` perform no conversion and reach the error path. The
Rust `strtol_base10` reproduces the C-locale whitespace set (space, `\t`, `\n`,
`\v`, `\f`, `\r`), the optional single sign, and the digit run.
See `successful_conversions_with_strtol_quirks`, `no_conversion_performed`.

### 5. `ERANGE` vs. the `int` range check are two different rejections

Values that overflow `long` set `errno = ERANGE` and fail the `errno == 0`
check; values that fit in `long` but not `int` pass that check and fail
`tmp >= INT_MIN && tmp <= INT_MAX`. Both end at the same `printf`, so the
distinction is invisible in the output — but it is a real pair of branches and
both are exercised, along with `LONG_MAX`, `LONG_MAX + 1`, `LONG_MIN`,
`LONG_MIN - 1`, `INT_MAX ± 1` and `INT_MIN ± 1`. The Rust clamps at `i64` and
reports an `erange` flag that gates `errno_is_zero`.
See `long_range_and_erange`, `int_range_boundaries`.

### 6. Signed overflow of `bedrooms`

`house->bedrooms += extra_bedrooms` is signed-integer overflow, i.e. UB in C,
and `run` is called **twice on the same house**, so with `x = INT_MAX` bedrooms
goes `5 → -2147483644 → 3`. The Rust uses `wrapping_add`, which matches what
gcc emits here — confirmed with the C compiled both without optimization flags
and at `-O3`. `add_floor` likewise uses `wrapping_add`, though `floors` only
ever reaches 4.
See `bedroom_accumulation_overflow`.

### 7. State shared across the two `run` calls

`run(&the_house, x)` is called twice on the same struct, so the second call
starts from the mutated values: floors `2→3→4`, bathrooms `2.5→3.5→4.5`,
bedrooms accumulating `x` twice. All eight `print_house` lines are pinned in
`both_binaries_run_and_c_output_is_the_documented_reference`.

### 8. `%.1f` formatting

`bathrooms` only ever takes 2.5, 3.5 and 4.5 — exactly representable in binary
and already at one decimal place — so `printf("%.1f")` and Rust's `{:.1}` cannot
disagree on rounding, and the program never calls `setlocale`, so the decimal
separator is `.` in both. No tie-breaking or locale difference is reachable.

### 9. stderr and exit status

Neither program writes to stderr on any input, and both always exit 0 —
including on the error path, since `main` ends in a single `return 0`. A test
asserts this explicitly rather than relying on the diff, because a stdout-only
comparison would let an exit-status regression through.
See `stderr_always_empty_and_exit_status_always_zero`.

## Fuzzing

Beyond the enumerated cases, `randomized_differential_fuzz` runs 600
deterministic pseudo-random inputs (xorshift64\*, fixed seed, no dependency)
drawn from the alphabet the parser reacts to, at lengths spanning the 99-byte
boundary. During investigation an additional ~11,000 inputs were compared,
including uniformly random bytes over all 256 values at lengths 0–300. Zero
mismatches.

## Files

- `translation/tests/differential.rs` — the differential suite (17 tests, none
  `#[ignore]`d, skipped or disabled).
- `c_src/` — untouched. Only the generated `c_src/build/` directory was added,
  which is the CMake output tree the build instructions call for; `main.c` and
  `CMakeLists.txt` are unmodified.
