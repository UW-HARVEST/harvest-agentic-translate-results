# Verification report — `c_src/src/main.c` → `translation/src/main.rs`

## How this was verified

Both programs are built and run as subprocesses; stdout, stderr and exit status
are compared byte for byte. The Rust code is never linked as a library.

| | command |
|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` → `c_src/build/driver` |
| Rust | `cd translation && cargo build --release` → `translation/target/release/driver` |
| Tests | `cd translation && cargo test` (20 tests, `tests/differential.rs`) |

`tests/differential.rs` builds the C reference itself if `c_src/build/driver` is
absent, so `cargo test` is self-contained. Nothing under `c_src/` is modified;
only the generated `build/` tree is created.

## Mismatches found

**None.** Every input class enumerated below produced identical stdout, stderr
and exit status. In addition to the checked-in tests, an out-of-tree
differential fuzzer was run over ~3,000 randomized inputs plus ~120 curated
ones, against the C program built both at `-O0` (the default CMake flags, which
is what the graded build uses) and at `-O3`; no divergence in either.

Because the report would otherwise be empty, the section below records the C
behaviours that *had to be matched* — each one is a place where a naive
translation would have diverged, and each has a dedicated test. These were
verified as correct in the existing Rust source rather than fixed.

### 1. `scanf` skips newlines; `fgets` would not

`scanf("%d", &data[i])` skips **any** leading whitespace — space, `\t`, `\n`,
`\v`, `\f`, `\r` — before looking for a digit. So `"1 2 3"`, `"1\n2\n3"` and
`"1\r\n2\x0b3"` are all the same input, and whitespace-only input is
indistinguishable from EOF.

A line-oriented reader (`fgets`/`BufRead::lines` + one parse per line) would
diverge on the very first multi-value-per-line input. `Scanner::scan_i32`
skips whitespace bytes directly instead.

Covered by `whitespace_only_is_eof`, `scanf_reads_across_newlines`.

Confirmed with the mutation `is_space(c) && c != b'\n'` (i.e. line-oriented
reading): the suite fails on `"4\n-5\n6\n0\n7"`.

### 2. `%d` converts via `long`, saturating, then truncates to `int`

glibc implements `%d` on LP64 by running a `strtol`-equivalent and assigning the
`long` result to an `int`. On overflow `strtol` **clamps** to `LONG_MIN` /
`LONG_MAX` (setting `ERANGE`) and the assignment then **truncates** the low 32
bits. The two steps do not commute:

| input | `long` after clamp | stored `int` | printed `x*x+x` |
|---|---|---|---|
| `2147483648` | `2147483648` | `-2147483648` | `-2147483648` |
| `4294967296` | `4294967296` | `0` | `0` |
| `9223372036854775807` | `LONG_MAX` | `-1` | `0` |
| `99999999999999999999999999` | `LONG_MAX` (clamped) | `-1` | `0` |
| `-99999999999999999999999999` | `LONG_MIN` (clamped) | `0` | `0` |

Note the asymmetry in the last two rows: a hugely positive literal yields `-1`
and a hugely negative one yields `0`. Reducing an arbitrary-precision value
modulo 2³² — the obvious translation — gives the wrong answer for anything past
the `long` range, and `i64::from_str` returning `Err` on overflow would give
the wrong answer too. `saturate_then_truncate` performs clamp-then-truncate in
that order.

Covered by `out_of_int_range_values`, `absurdly_long_digit_runs`,
`full_array_of_extremes`.

Confirmed with the mutation that drops the `LONG_MIN` clamp: the suite fails on
`"-99999999999999999999999999"`.

Related detail: digit accumulation is guarded by `OVERFLOW_GUARD` so a 5,000-digit
run cannot overflow the accumulator itself. The guard stops accumulating once the
magnitude already exceeds `i64::MAX`, which is safe precisely because the result
is clamped anyway.

### 3. Signed overflow in `out[i] * out[i] + out[i]`

`fma_array` is called as `fma_array(out, out, out, out, len)` — all four
pointers alias the same buffer — so each element becomes `x*x + x` computed from
its own current value. Signed overflow is UB in C; the compiled code uses `imul`
and wraps, at `-O0` and `-O3` alike. The Rust uses `wrapping_mul` /
`wrapping_add`, so e.g. `46341` prints `-2147383295` rather than panicking (a
debug build with plain `*` would abort) or saturating.

Covered by `signed_overflow_in_the_fma`.

### 4. The `i < 100` bound, and what happens at exactly 100

`main` reads into `int data[100]` and the loop has two exits: the count reaching
100, or `scanf` returning anything other than 1. Reaching 100 exits **without
consuming another token**, so trailing garbage after the 100th value is never
examined and can never turn into an error — input 101, 150 or 500 values, or
100 values followed by `"not-a-number"`, and the output is the same 100 lines.
An off-by-one in the bound is invisible on small inputs.

Covered by `item_count_boundaries` (0, 1, 2, 98, 99, 100, 101, 102, 150, 500),
`garbage_after_hundred_items_is_never_read`,
`matching_failure_at_every_boundary`.

Confirmed with the mutation `while i < 99`: the suite fails on the 100-item case.

### 5. Matching failure and EOF are the same branch

`scanf` returns `0` on a matching failure and `EOF` (`-1`) on end of input.
`main` checks `!= 1`, so both take the same `break` and the distinction never
surfaces. `scan_i32` collapses both to `None`. Only the values read *before* the
bad token are transformed and printed — `"1 2 x 3 4"` prints two lines, not
four. A sign with no digits after it (`"-"`, `"+"`, `"- 5"`, `"--5"`) is a
matching failure, and `"12abc"` yields `12` and then fails on the next call.

Covered by `matching_failure_truncates_the_input`,
`sign_without_digits_is_a_matching_failure`, `eof_immediately_after_digits`.

### 6. `int i` is left holding the count, and `len == 0` is silent

`i` is declared outside the loop, so after the loop it is the number of values
successfully read, and that is what is passed as `len`. With `len == 0` both
loops in `driver` run zero times: **no output at all**, not a blank line, and
`return 0`. The uninitialised tail of `data[100]` is never read back, so the
Rust zero-initialised array is not observable.

Covered by `empty_input_prints_nothing`, `whitespace_only_is_eof`.

### 7. Output format, exit status, and stderr

`printf("%d\n", ...)` — one value per line, decimal, no padding, no separator,
and a trailing newline on the final line too. The program writes nothing to
stderr and always returns `0`, including for every error path above: there is no
input that makes it exit non-zero. A test that only diffed stdout would pass a
Rust program that exited 1, so the suite asserts the exit status on every case.

Confirmed with the mutation `std::process::exit(1)`: the suite fails on the
empty-input case with `C 0 vs Rust 256`.

## Input classes enumerated from the C source

Every `if`, early exit and branch in `main` / `driver` / `fma_array`:

- empty input; whitespace-only input (each of the six `isspace` bytes, and a
  10,000-byte run)
- 1 value; 2 values; 98, 99, **100**, 101, 102, 150, 500 values
- separators: space, `\n`, `\t`, `\r\n`, `\v`, `\f`, mixed runs, none
- with and without a trailing newline; leading whitespace before the first value
- matching failure at position 0, 1, 2, 50, 98, 99, 100, 101
- non-numeric tokens: letters, `.`, `,`, `;`, `_`, `3.5`, `1e5`, `inf`, `nan`,
  `0x10`, `12abc`, `0junk`, U+00B1
- sign-only / sign-without-digits: `-`, `+`, `-a`, `+a`, `- 5`, `--5`, `+-5`,
  `-.`, `-\n5`
- accepted spellings: `+7`, `-0`, `+0`, leading zeros, `010`..`019` (decimal,
  not octal), a 500-zero run, all-zeros
- embedded NUL: alone, after a value, between values, inside a digit run
- `int` boundaries: ±2147483647, −2147483648, ±2³², 2³²±1
- `long` boundaries: ±(2⁶³−1), 2⁶³, −2⁶³−1, 2⁶⁴−1, 2⁶⁴
- beyond `long`: 19/20/21/40/100/500/5000-digit runs, positive and negative,
  zero-padded variants
- fma overflow: 0, ±1, ±2, ±3, ±32768, 46339/46340/**46341**, ±65536, 100000,
  2³⁰, ±2³⁰, 2147483646/47, −2147483647/48
- a full 100-element array of the extreme values above
- 400 deterministic pseudo-random cases (fixed seed) mixing all token kinds,
  separators and counts from 0 to 105
- stdin closed rather than empty (checked manually; both exit 0 with no output)

## Completion gate

- Both programs build with no errors — yes (`cmake --build .`; `cargo build
  --release` clean).
- Every enumerated input gives identical stdout, stderr and exit status — yes.
- `cargo test` passes in `translation/` — yes, 20/20, in both the debug and
  release profiles.
- No test disabled, skipped or `#[ignore]`d — none; there is no `#[ignore]`,
  `return`-early or conditional-skip anywhere in `tests/`.
- Nothing in `c_src/` modified — `src/main.c` and `CMakeLists.txt` are
  untouched; only the generated `build/` directory was added.
