# ERRORS.md — differential verification of the C → Rust translation

Reference C: `c_src/src/main.c` (built by `c_src/CMakeLists.txt` as `driver`).
Rust under test: `translation/src/main.rs` (binary `driver`).

Both programs were built and driven as subprocesses with identical `argv`;
stdout bytes, stderr bytes and exit status were compared on every case.

## Commands used

```
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver

# Rust
cd translation && cargo build --release                                 # -> translation/target/release/driver

# Differential suite
cd translation && cargo test
```

## Branches enumerated from the C source

`main` has exactly three exits and one loop; `static_sum` has none.

| # | C construct | Input class | Test |
|---|---|---|---|
| 1 | `if (argc != 2)` | no argument (`argc == 1`) | `argc_zero_extra_args` |
| 2 | `if (argc != 2)` | 2+ arguments (`argc == 3, 4, 12`) | `argc_two_extra_args`, `argc_many_extra_args` |
| 3 | `if (end == argv[1])` | `strtol` performs no conversion: empty string, whitespace only, sign only, non-numeric, non-ASCII digits | `no_conversion_*` (5 tests) |
| 4 | fallthrough to the loop | any string `strtol` converts at least one digit from | `happy_path_*` (4 tests) |
| 5 | `int stride = strtol(...)` | `long` → `int` truncation | `long_to_int_truncation` |
| 6 | `strtol` ERANGE | magnitude beyond `LONG_MAX` / `LONG_MIN`, clamped then truncated | `strtol_range_clamping`, `strtol_range_clamping_very_long_inputs` |
| 7 | `i * stride` | signed multiply overflow inside the loop | `int_multiply_overflow_in_loop` |
| 8 | `sum += update` (function-local `static`) | running total crossing `INT_MAX` / `INT_MIN` (sum is `45 * stride`) | `running_sum_overflow_boundary` |
| 9 | `argv` is `char **` | argument bytes that are not valid UTF-8 | `non_utf8_arguments` |
| 10 | errors use `printf`, not `fprintf(stderr, ...)` | error messages must land on **stdout**, stderr stays empty | `error_paths_write_to_stdout_not_stderr_and_exit_1` |
| 11 | no `scanf` / `fgets` anywhere | stdin must be ignored entirely | `stdin_is_never_read` |

## Mismatches found

**None.** No input produced a difference in stdout, stderr or exit status
between the C and the Rust program. The Rust source in `translation/src/main.rs`
was not modified during verification.

Beyond the 21 tests in `translation/tests/differential.rs`, an ad-hoc fuzz run
compared the two binaries on **2,711** additional invocations with byte-exact
comparison, all matching:

- 1,511 cases: random integers at 8/16/31/32/33/63/64/70/100-bit magnitudes with
  random sign, leading whitespace, explicit `+`, leading-zero padding and
  trailing garbage; random printable-ASCII garbage strings; random `argv` arity
  0–4; every stride in `-60..=60` exhaustively; and `±3` around `2^31`, `2^32`,
  `2^63`, `2^64` and `45 * 2^31`.
- 1,200 cases: random raw non-NUL byte strings (1–10 bytes) and random mixes of
  `'5' '-' '+' ' ' '0' '9' '\t' 0xff 0x80 0xc3`, passed as raw `argv` bytes.

## Behaviors that had to be reproduced (verified correct, not fixed)

Recorded because each is a place a naive translation would diverge:

1. **Errors go to stdout, not stderr.** The C uses `printf` for both error
   messages, so stderr is always empty even on the failure paths. A translation
   using `eprintln!` would pass a stdout-only test and still be wrong.
2. **`strtol` semantics, not `str::parse`.** `strtol` skips leading
   `isspace()` bytes, accepts an optional sign, and stops at the first
   non-digit *without* erroring. So `"5abc"`, `"5.9"`, `"0x10"` (base 10 → parses
   `0`, stops at `x`) and `"  -42xyz"` are all **accepted**. `str::parse::<i32>()`
   rejects every one of them and would take the error branch instead.
3. **`end == argv[1]` is the only error condition.** The check is "was anything
   converted", not "was the whole string consumed". Trailing garbage is fine;
   `"  -  5"` (space between sign and digits) is not.
4. **`long` → `int` truncation.** `strtol` returns `long`; assigning to
   `int stride` truncates. `"4294967296"` → stride `0`, `"4294967295"` → `-1`.
5. **ERANGE clamping happens before truncation.** Out-of-range magnitudes clamp
   to `LONG_MAX` / `LONG_MIN` first, and *then* truncate: `"99999...9"` →
   `(int)LONG_MAX` → `-1`, and `"-99999...9"` → `(int)LONG_MIN` → `0`. Truncating
   a saturated 32-bit value instead would give `INT_MAX` / `INT_MIN` and differ.
   `end` still advances past every digit, so the long-input cases take the
   success path.
6. **Signed overflow wraps.** `i * stride` and the accumulating `sum` overflow
   for large strides. This is UB in C, so it is not guaranteed — but the
   reference binary wraps, and the Rust uses `wrapping_mul` / `wrapping_add` to
   match. Confirmed the reference does not diverge under optimization either: a
   throwaway `-O3` build of `c_src` (out-of-source, in `/tmp`, `c_src` untouched)
   agreed with the Rust on all overflow inputs.
7. **Non-UTF-8 `argv`.** The C only ever sees bytes. The Rust reads `args_os()`
   and takes the raw bytes via `OsStrExt`, so `"5\xff"` parses as `5` rather
   than being rejected or lossily replaced.
8. **`static int sum` persists across calls within one process.** The ten
   printed values are a running total (`0 1 3 6 10 15 21 28 36 45` for stride 1),
   not ten independent products. Each process starts fresh at `0`.

## Non-mismatch: one untestable input class removed from the suite

An initial case passed `"\0"` as the argument, intending to exercise "NUL byte in
`argv`". It failed at spawn time (`nul byte found in provided data`) in the test
harness, not in either program. This input class does not exist: `execve`
terminates each argument at the first NUL, so no process can ever observe a NUL
inside `argv[1]`. The case was deleted and a comment left in its place. No
program behavior was involved, and nothing was disabled or `#[ignore]`d to make
the suite pass.
