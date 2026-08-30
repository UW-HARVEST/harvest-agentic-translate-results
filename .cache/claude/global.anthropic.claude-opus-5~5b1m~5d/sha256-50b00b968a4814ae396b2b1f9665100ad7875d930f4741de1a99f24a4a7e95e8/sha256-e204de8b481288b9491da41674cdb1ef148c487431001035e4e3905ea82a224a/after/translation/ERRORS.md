# Differential verification log — `c_src/src/main.c` vs `translation/`

## Summary

**No behavioural mismatches were found.** Across ~33,000 differential executions
(hand-enumerated branch cases, exhaustive short-token sweeps, and seeded random
fuzzing) the Rust binary produced byte-identical stdout, byte-identical stderr,
and an identical exit status to the C binary for every input.

No change to `translation/src/main.rs` was required. Nothing in `c_src/` was
modified (only `c_src/build/`, the CMake output directory, was created).

Because there is no list of "mismatches I fixed", this document instead records
**every place the two implementations could have diverged, the inputs used to
probe it, and the evidence that they agree.** That is the checkable artifact: a
reader can re-run each harness below and reproduce the result.

---

## How to reproduce

```
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver

# Rust
cd translation && cargo build --release                                 # -> translation/target/release/driver

# The graded comparison
cd translation && cargo test

# The wider exploratory sweeps (Python, ~33k subprocess pairs)
cd translation/tools
python3 cmp.py        # 84 hand-picked branch cases
python3 cmp2.py       # 232 strtod / fgets-boundary cases
python3 cmp3.py       # 2,091 cases incl. exhaustive 1-3 char tokens
python3 sweep4.py     # 6,561 exhaustive 4-char tokens over "0.exni-f9"
python3 fuzz.py <seed>  # 4,000 random cases per seed; seeds 1-11 all clean
```

---

## What the C program actually does (the branch inventory)

`main` is fixed and takes no input: it prints `Calling good()...`, runs
`goodG2B()` then `goodB2G()`, prints `Finished good()`, prints
`Calling bad()...`, runs `bad()`, prints `Finished bad()`, returns 0.

Only **two** stdin reads ever happen, in this order:

1. `goodB2G()`'s `fgets(buf, 20, stdin)`
2. `bad()`'s `fgets(buf, 20, stdin)`

`goodG2B()` reads nothing and always prints `50`. Everything after line 3 of
input is never read.

Branch points, all of which have a dedicated test:

| # | Branch | Input class | Test |
|---|---|---|---|
| 1 | `goodB2G`: `fgets != NULL` | ≥1 byte on stdin | `fgets_null_and_non_null_branches` |
| 2 | `goodB2G`: `fgets == NULL` → `"fgets() failed."` | empty stdin | same |
| 3 | `bad`: `fgets != NULL` | ≥2 lines | same |
| 4 | `bad`: `fgets == NULL` → `"fgets() failed."` | 0 or 1 line | same |
| 5 | `goodB2G`: `fabs(data) > 0.000001` true | e.g. `4` | `goodb2g_divide_by_zero_guard_both_branches` |
| 6 | `goodB2G`: false → `"This would result in a divide by zero"` | `0`, `-0`, `abc`, `nan`, `1e-7` | same |
| 7 | `bad`: unguarded `(int)(100.0/data)` with `data == 0` | `5\n0\n` | `bad_divides_unconditionally_including_by_zero` |
| 8 | `printLine`: `line != NULL` | always (every call site passes a literal) | implicit |
| 9 | `printLine`: `line == NULL` | **unreachable** — no call site passes NULL | n/a, documented |

Branch 9 is dead code in the C. The Rust models it as `Option<&str>` and every
call site passes `Some(...)`, so the dead branch is dead in both. No input can
reach it, so no test can exist for it.

---

## Risk areas probed, and why they agree

### 1. `(int)` cast of infinity and NaN — undefined behaviour in C

`bad()` has no divide-by-zero guard, so `data == 0.0f` gives
`100.0 / 0.0 == +inf`, and `(int)+inf` is UB. The C binary (gcc 11.5, x86-64)
compiles this to `cvttsd2si`, which returns the "integer indefinite" value
`0x80000000` = **-2147483648** for infinities, NaN, and every out-of-range
magnitude.

The Rust does **not** use a bare `as i32` cast, which in Rust is *saturating*
(`inf as i32` would give `2147483647` — the opposite sign, a mismatch waiting to
happen). It instead uses `f64_to_int`, which returns `i32::MIN` for NaN and for
anything outside `[-2^31, 2^31-1]`. This is the single most important
UB-replication in the translation and it is correct.

Verified with `0`, `-0`, `-0.0`, `nan`, `-nan`, unparseable text, an empty line,
and a failed `fgets` — all print `-2147483648`, and the boundary was walked
digit by digit around `100.0/2147483647 ≈ 4.6566128e-8`
(`int_cast_truncation_and_overflow`). `+inf`/`-inf` *input* takes the other
path: `100.0/inf == 0.0` → prints `0`.

### 2. Truncation direction of the `(int)` cast

C truncates toward zero, not toward negative infinity. `-3` → `100/-3 =
-33.33` → `-33`, not `-34`. Rust's `.trunc()` matches. Swept over every
integer in `[-40, 40]` in `deterministic_sweep_over_generated_inputs`.

### 3. `fgets` reads at most 19 bytes and keeps the newline

`CHAR_ARRAY_SIZE` is 20, so `fgets` consumes at most 19 bytes and does **not**
skip to the end of the line. A first line longer than 19 bytes is therefore
split: the tail is what `bad()`'s `fgets` sees. This is the "looks like a bug,
is not a bug" behaviour the task warns about.

Example, pinned by both binaries: `111111111111111111112222\n` →
`goodB2G` parses `1111111111111111111` (1.111e18, prints `0`), and `bad()`
parses the remaining `12222\n` (prints `0`). The Rust `fgets` helper stops at
`buffer.len() - 1`, keeps the `\n`, appends a NUL, and returns "NULL" only when
zero bytes were read — matching C exactly.

Every line length from 0 to 26 bytes is tested individually, in three shapes
(with a following line, alone with a newline, alone without one), in
`fgets_nineteen_byte_buffer_boundary`.

### 4. `fgets` returning NULL leaves `data` at its initialiser

On EOF-with-no-bytes, C's `fgets` returns NULL and does not touch the buffer;
`data` keeps the `0.0F` assigned before the block. So empty stdin makes
`goodB2G` print the divide-by-zero message and `bad()` print `-2147483648`.
The Rust returns `false` without writing to the buffer and leaves `data` at
`0.0f32`. Confirmed for empty stdin, `/dev/null`, and a closed fd 0.

### 5. `atof` == `strtod` semantics

`atof` is not a strict parser, and every one of its quirks matters because they
decide whether `data` is 0.0 (divide-by-zero branch) or not:

- leading whitespace is skipped — all six C space characters
  (`' '`, `\t`, `\n`, `\v`, `\f`, `\r`) tested
- trailing junk is ignored, not an error: `12abc` → 12.0
- a completely unparseable string is **0.0, not an error**: `abc` → 0.0 →
  divide-by-zero branch
- `inf`, `INF`, `infinity`, `INFINITY`, and mixed case are accepted;
  `i`, `in`, `infin` are not (→ 0.0)
- `nan`, `NaN`, and the `nan(n-char-seq)` form are accepted
- hex floats (`0x1p3`, `0x1.8p1`, `0x.8p2`) are accepted; a bare `0x` converts
  just the `0` and yields 0.0
- `5.` and `.5` are valid; `.` and `.e5` are not
- a truncated exponent (`1e`, `1e+`) is not part of the conversion, so `1e`
  → 1.0
- `1_000` → 1.0 (C has no digit separators)
- overflow saturates to inf (`1e999`), underflow to 0.0 (`1e-999`)

The Rust reimplements `strtod` by finding the longest valid prefix and handing
it to Rust's correctly-rounded `f64::from_str`, with separate paths for
inf/nan/hex. One subtlety worth noting: the prefix can be `"5.e3"`, which C
accepts and — verified — Rust's `FromStr` also accepts, so no special-casing is
needed. Exhaustive sweeps of all 1-, 2-, 3-character tokens over
`0.expni+-f a` and all 4-character tokens over `0.exni-f9` found no
disagreement.

### 6. `float` vs `double` precision, and the epsilon guard

`data` is a `float`, but `atof` returns a `double` and the division is done in
`double`. The Rust mirrors this exactly: `c_atof(...) as f32`, then
`data as f64` for the division and for `fabs`.

This double narrowing has an observable consequence at the guard: the input
`0.000001` becomes `1e-6f32`, which as a `double` is `1.0000000116...e-6`,
which **is** greater than the literal `0.000001` double. So `0.000001` takes the
*true* branch. Both binaries agree; the test comments record this so a future
reader does not "fix" it. Probed with 60 values swept across the epsilon and
with `9.9999994e-7` / `9.9999995e-7`, which straddle the float rounding step.

Float overflow (`1e39` → `+inf`) and float flush-to-zero (`1e-46` → `0.0f` →
divide by zero) were also confirmed identical.

### 7. Embedded NUL bytes truncate the string, but not the read

`fgets` happily stores a NUL byte, but `atof` stops there. `5\x006` → 5.0.
The Rust `c_atof` truncates at the first NUL before parsing. Also verified with
a leading NUL, a lone NUL, an all-NUL buffer, invalid UTF-8 (`\xff\xfe`), lone
UTF-8 continuation bytes, and an all-`0xff` buffer — the Rust never panics on
non-UTF-8 input because it works on `&[u8]` throughout and only uses
`from_utf8_lossy` on an already-validated ASCII numeric prefix.

### 8. CRLF

`fgets` keeps the `\r`, and `strtod` stops at it, so `5\r\n` still parses as 5.
A lone `\r` is *not* a line terminator for `fgets`, so `5\r4\r` is one line.
Both confirmed.

### 9. `printf` formatting and trailing newlines

`printLine` is `printf("%s\n", ...)` and `printIntLine` is `printf("%d\n", ...)`.
`println!` matches for both. Every test compares raw bytes, and
`printf_formatting_is_byte_identical` additionally asserts the output ends with
exactly one `Finished bad()\n` and no doubled trailing newline.

### 10. Exit status, stderr, and argv

`main` always `return 0`, writes nothing to stderr, and ignores `argc`/`argv`.
Verified: exit status 0 and empty stderr on every one of the ~33,000 cases;
extra command-line arguments (`hello`, `a b c`, `-h --help`) change nothing.

### 11. Buffering and stream-teardown differences

C's stdout is fully buffered on a pipe; Rust's is line-buffered. This is
unobservable here because stderr is never written, so there is no interleaving
to get wrong. Checked anyway: closed stdout (`>&-`) and a truncated pipe
(`| head -c 5`) produce identical behaviour and exit status from both programs.

---

## Things deliberately *not* changed

- `f64_to_int` returns `i32::MIN` for out-of-range values. This looks like a
  magic constant, but it is the whole point: it reproduces the x86-64 `cvttsd2si`
  result for C's UB. Replacing it with `as i32` (saturating) or with a
  `checked` conversion would break the `-2147483648` outputs.
- The dead `printLine(NULL)` branch is kept as `Option<&str>`.
- `goodG2B` keeps its unconditional `50`.

## Residual risk

The `-2147483648` outputs depend on C UB, so they are a property of *this*
compiler and *this* architecture (gcc 11.5.0, x86-64), not of the C standard.
Both programs were built and compared on the same machine, which is the
condition under which the translation is graded. On a target whose
float→int conversion traps or saturates differently, the C binary itself would
change, and `f64_to_int` would need to change with it.

The `parse_hex_float` mantissa-truncation path (`digits < 28`) is unreachable in
practice: `fgets` caps input at 19 bytes, so a hex literal can carry at most 17
hex digits. It is retained as defensive code.
