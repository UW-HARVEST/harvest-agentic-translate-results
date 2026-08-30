# Differential verification: C (`c_src/`) vs Rust (`translation/`)

Ground truth is `c_src/src/main.c`. The Rust binary must produce byte-identical
stdout, stderr and exit status for every input.

## Result

**No mismatches were found.** The translation in `src/main.rs` already agreed
with the C on every input exercised — the enumerated cases in
`tests/differential.rs` plus roughly 6,900 additional ad-hoc and fuzzed inputs
run during investigation.

Because "no mismatch found" is only as strong as the inputs tried, the coverage
evidence is recorded below, followed by the specific behaviours that were
verified rather than assumed. Those are the places a translation of this program
would plausibly go wrong.

## Coverage evidence

`main.c` compiled with `gcc --coverage -O0` and driven with the enumerated input
classes reports full coverage:

```
Lines executed:100.00% of 37
Branches executed:100.00% of 10
Taken at least once:100.00% of 10
Calls executed:100.00% of 14
```

The only real branch point is the four-way conjunction in `parse_val`:

```c
if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX)
```

gcov confirms each of the four conjuncts was observed both true and false, so
every way this program can reject an input is covered, not just one of them.

## Behaviours verified, not assumed

Each of these is a point where the C does something surprising and where a
plausible Rust translation would diverge. All were confirmed identical.

### 1. `fgets` does not read across newlines

`fgets(in, sizeof(in), stdin)` reads one line only; the rest of stdin is
discarded. A translation using `read_to_string` or `scanf`-like semantics would
differ on `"3\n7\n"` — the C parses `3` and never sees the `7`.
Covered by `fgets_stops_at_first_newline`.

### 2. The buffer is 100 bytes, so input is cut at 99

`in` is `char[100]`, so at most 99 bytes are read and the newline may never
arrive. A number straddling that boundary is silently truncated: 98 spaces
followed by `12345` parses as **1**, not 12345, because only the first digit
fits. `translation/src/main.rs` reproduces this with `fgets(100)` reading while
`out.len() + 1 < size`.
Covered by `fgets_99_byte_truncation`, including newlines landing at bytes 99,
100 and 101.

### 3. The buffer is consumed as a C string, so an embedded NUL ends it

`char in[100] = ""` zero-fills, and `strtol` stops at the first NUL. Input
`"\0" + "12"` therefore parses nothing and prints the error, even though `fgets`
did read the digits into the buffer. The Rust truncates `raw` at the first NUL
byte before parsing, which is what makes this match.
Covered by `embedded_nul_terminates_the_c_string`.

### 4. Empty stdin leaves the buffer valid, not undefined

On immediate EOF `fgets` returns NULL and does not touch `in`, which is still
`""` from its initialiser. `strtol("")` converts nothing, `endp == str`, so the
program prints `An error occurred` and still returns **0**. The Rust returns an
empty `Vec` from its `fgets`, reaching the same path.
Covered by `empty_and_eof_input`.

### 5. Trailing garbage is accepted

`parse_val` only requires `endp != str` — that at least one digit converted. It
never checks that the whole string was consumed. So `"12abc"` succeeds with 12,
`"2.9"` succeeds with 2, and `"0x10"` succeeds with 0 (base 10 stops at `x`).
A translation using Rust's `str::parse::<i32>()` would reject all three.
Covered by `trailing_garbage_is_accepted`.

### 6. Rejection has two distinct causes that must not be conflated

- **Out of `int` range but in `long` range** — `"2147483648"`, `"9223372036854775807"`:
  `strtol` succeeds and sets no errno; the `tmp <= INT_MAX` guard rejects.
- **Out of `long` range** — `"9223372036854775808"`: `strtol` saturates to
  `LONG_MAX` *and* sets `ERANGE`; the `errno == 0` guard rejects.

Both print the same message, so a test checking only stdout cannot tell them
apart — but a translation that clamped instead of failing, or that failed
instead of clamping, would break on one side. `"-9223372036854775808"` is the
delicate case: it is exactly `LONG_MIN`, converts with **no** `ERANGE`, and is
rejected purely by the `tmp >= INT_MIN` guard. The Rust `strtol_base10` uses a
separate magnitude limit of `i64::MAX as u64 + 1` when negative to get this
right.
Covered by `int_range_boundaries`, `long_range_boundaries_and_erange`,
`every_int_boundary_value_exhaustively`.

### 7. Leading zeros do not cause overflow

80 zeros followed by `2147483647` is accepted, because `strtol` tracks the value
and not the digit count. Any translation that pre-checked input length would
wrongly reject it.
Covered by `leading_zeros_do_not_trigger_overflow`.

### 8. Signed `int` overflow in `add_bedrooms`

`bedrooms` starts at 5, so `5 + INT_MAX` overflows. This is undefined behaviour
in C; gcc wraps in practice. Confirmed that `-O0` and `-O3` builds of the C
produce identical output here, and that both match the Rust, which uses
`wrapping_add` deliberately. Because `run()` is called twice, the value wraps
and then wraps back — `2147483647` gives bedrooms `-2147483644` then `3`.
Covered by `global_state_persists_across_both_runs`.

### 9. Global state persists between the two `run()` calls

`the_house` is a mutable global, so the second `run()` continues from the first:
floors go 2→4 and bathrooms 2.5→4.5 across the eight printed lines. A
translation that reinitialised state per call would produce eight lines of
plausible-looking but wrong output.
Covered by `output_shape_is_exactly_what_the_c_prints`, which pins the exact
expected bytes.

### 10. `%.1f` formatting

`bathrooms` only ever takes the values 2.5, 3.5 and 4.5. All three are exactly
representable, so glibc's round-half-to-even and Rust's `{:.1}` cannot disagree,
and the locale decimal separator is not a factor because the C never calls
`setlocale`. The NaN/infinity arms of `format_f1` in `src/main.rs` are therefore
unreachable dead code, kept only for faithfulness.

### 11. Whitespace, and what is not whitespace

`strtol` skips leading whitespace per the C locale — space, tab, newline,
vertical tab, form feed, CR. Bytes `0xFF`/`0xFE` are **not** whitespace and are
rejected. A leading `\n` is especially easy to get wrong: it is skipped by
`strtol`, but `fgets` already stopped at it, so `"\n5\n"` never sees the `5` and
errors.
Covered by `leading_whitespace_is_skipped`, `no_conversion_is_an_error`.

### 12. Exit status is unconditionally 0, and stderr is always empty

`main` ends in `return 0` on both paths, and the error message goes to
**stdout** via `printf`, not stderr. A translation that used `eprintln!` or
`std::process::exit(1)` for the error path would produce identical-looking
terminal output while failing both the stderr and exit-status comparisons.
Covered by `errors_go_to_stdout_not_stderr_and_exit_is_always_zero`.

## Environment-level paths checked outside the test suite

These depend on how the process is invoked rather than on stdin content, so they
are recorded here rather than as tests. All matched.

| Condition | C | Rust |
|---|---|---|
| stdin closed (`<&-`) | `An error occurred`, exit 0 | identical |
| stdin is a directory | `An error occurred`, exit 0 | identical |
| stdout is `/dev/full` (writes fail ENOSPC) | silent, exit 0 | identical |
| stdout is a closed pipe | exit 0 | identical |

The `/dev/full` case matters because the C ignores every `printf` return value;
the Rust matches by discarding write results with `let _ =`. The closed-pipe
case cannot actually raise `SIGPIPE` here — the whole output is 8 lines and fits
in the pipe buffer — so Rust's default `SIGPIPE` disposition of `SIG_IGN` is not
observable for this program.

## How to reproduce

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
cd ../../translation && cargo build --release && cargo test
```

`tests/differential.rs` builds the C program itself via CMake if
`c_src/build/driver` is absent, then compares the two binaries as subprocesses.
Nothing in `c_src/` is modified; only the `build/` output directory is created.
