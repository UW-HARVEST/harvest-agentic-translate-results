# ERRORS.md — Phase A: error / rejection surface table

Derived mechanically from `c_src/src/main.c`. The program contains **no**
`RETURN_ERROR`-style macros, **no** `assert`, **no** `return -1` / `return NULL`,
**no** error enums, **no** null checks and **no** explicit range checks — the
complete grep result for rejection-ish constructs is:

```
$ grep -nE 'assert|return|NULL|errno|exit|ERROR|if \(|while \(' c_src/src/main.c
28:static void foo(int x, int y) {
29:    while (x > 0 || y > 0) {         <- the only "reject the whole workload" gate
32:        if (x == 1 && y == 4) {
37:        if (x > 0) {
43:        if (y == 0) {
48:        if (x < 3) {
55:    scanf("%d %d", &x, &y);          <- return value DISCARDED
57:    return 0;                        <- the only return; always 0
```

So the entire rejection surface is (a) the ways `scanf` can fail to convert,
whose only effect is that a variable keeps its `int x = 0, y = 0;` default
because the return value is discarded, (b) the loop guard rejecting non-positive
workloads, (c) integer conversions that saturate/truncate, (d) unchecked output
failures, and (e) signed-overflow UB. Each distinct rejection gets one row.

`printf` is compiled to `puts` (see `SYMBOLS.md`) and its return value is never
checked either.

Values in the "expected C result" column were confirmed empirically against the
compiled C (and, for the glibc `%d` conversion semantics, with a standalone
`fscanf` probe): glibc converts `%d` via `strtol` into a `long`, **saturating**
at `LONG_MAX`/`LONG_MIN` on overflow, then stores it with `*ARG(int *) = num.l`,
i.e. a silent truncation to 32 bits.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✓ |
|----|----------|---------------------------------------------|-------------------|------|---|
| E1 | `main`/`scanf` conv #1 | **input failure**: stdin is empty (immediate EOF) | `scanf` returns `EOF`; `x`,`y` keep `0`; `foo(0,0)` prints nothing; exit 0 | `e1_conv1_input_failure_empty` | [x] |
| E2 | `main`/`scanf` conv #1 | **input failure**: stdin is whitespace only (`" "`, `"\n"`, `"\t\v\f\r"`) — `%d` skips it all and hits EOF | `EOF`; `x=y=0`; no output; exit 0 | `e2_conv1_input_failure_whitespace_only` | [x] |
| E3 | `main`/`scanf` conv #1 | **matching failure**: first non-whitespace byte cannot start an integer (`"abc"`, `"x 1"`, `".5"`, `"/"`) | returns `0`; `x`,`y` keep `0`; no output; exit 0 | `e3_conv1_matching_failure_nondigit` | [x] |
| E4 | `main`/`scanf` conv #1 | **matching failure after sign**: `"-"`, `"+"`, `"- 5"`, `"+x"` — sign consumed, no digit follows | returns `0`; `x`,`y` keep `0`; no output; exit 0 | `e4_conv1_matching_failure_sign_only` | [x] |
| E5 | `main`/`scanf` conv #1 | **matching failure on NUL / high byte**: leading `0x00`, or `0x80`–`0xFF` (not `isspace`, not a digit in the C locale) | returns `0`; `x=y=0`; no output; exit 0 | `e5_conv1_matching_failure_nul_and_high_bytes` | [x] |
| E6 | `main`/`scanf` conv #2 | **input failure**: EOF after the first integer (`"5"`, `"5 "`, `"5\n"`) — conv #2 never gets a character | returns `1`; `x=5`, `y` keeps `0`; runs `foo(5,0)` | `e6_conv2_input_failure_eof` | [x] |
| E7 | `main`/`scanf` conv #2 | **matching failure**: non-integer follows the first int (`"5 abc"`, `"5abc"`, `"5.5"`, `"5 x"`) | returns `1`; `x=5`, `y` keeps `0`; runs `foo(5,0)` | `e7_conv2_matching_failure_nondigit` | [x] |
| E8 | `main`/`scanf` conv #2 | **matching failure after sign**: `"5 -"`, `"5 +"`, `"5 -x"` | returns `1`; `x=5`, `y=0`; runs `foo(5,0)` | `e8_conv2_matching_failure_sign_only` | [x] |
| E9 | `main`/`scanf` `%d` | **overflow above `LONG_MAX`**: `"9223372036854775808"`, `"99999999999999999999999999"`, `"18446744073709551616"`, 1000-digit runs | `strtol` saturates to `LONG_MAX`, truncated to `int` → **`-1`** (loop guard then rejects it as non-positive) | `e9_overflow_above_long_max` | [x] |
| E10 | `main`/`scanf` `%d` | **overflow below `LONG_MIN`**: `"-9223372036854775809"`, `"-99999999999999999999999999"` | saturates to `LONG_MIN`, truncated to `int` → **`0`** | `e10_overflow_below_long_min` | [x] |
| E11 | `main`/`scanf` `%d` | **in `long` range but out of `int` range** (silent truncation, no error): `2147483648` → `-2147483648`; `-2147483649` → `2147483647`; `4294967296` → `0`; `9223372036854775807` → `-1` | truncated value used by `foo`; no error signalled | `e11_int_truncation_in_long_range` | [x] |
| E12 | `main`/`scanf` `%d` | **`int` boundary values exactly**: `2147483647` (`INT_MAX`), `-2147483648` (`INT_MIN`) | converted exactly; `INT_MIN` for `y` enters the overflow path of E15 | `e12_int_boundaries_exact` | [x] |
| E13 | `main`/`scanf` `%d` | **no locale digit grouping** (`%d`, not `%'d`): `"1,000 5"` → conv #1 stops at `,`; conv #2 then fails on `,` | returns `1`; `x=1`, `y` keeps `0`; runs `foo(1,0)` | `e13_no_digit_grouping` | [x] |
| E14 | `foo` | **loop guard rejects the workload**: `!(x > 0 \|\| y > 0)`, i.e. `x<=0 && y<=0` — includes `(0,0)`, `(-n,0)`, `(0,-n)`, `(-n,-m)`, `(INT_MIN,INT_MIN)` | zero iterations; **empty stdout**; exit 0 | `e14_loop_guard_rejects_nonpositive` | [x] |
| E15 | `foo` | **signed-overflow UB**: `y--` executes whenever `y != 0`, so a negative `y` is decremented past `INT_MIN`. As compiled (`gcc -O0`) it wraps to `INT_MAX`; reached whenever `x>0 && y<0` | no diagnostic, no trap: an **unbounded** run (~2^32 `"y\n"` lines) until `y` wraps back to `0`. Compared by fixed-length output **prefix** | `e15_signed_overflow_unbounded_prefix` | [x] |
| E16 | `foo` | **`x--` cannot overflow**: guarded by `if (x > 0)`, so `INT_MIN`/`0` never decrement. `x=INT_MIN` must *not* print `"x\n"` | `foo(INT_MIN, y>0)` prints `loop` then only `y` lines | `e16_x_decrement_guarded` | [x] |
| E17 | `main`/`printf` | **output write failure is unchecked**: stdout is `/dev/full` (every write fails `ENOSPC`) | return value of `printf` discarded; program still **exits 0**; no bytes delivered | `e17_stdout_write_failure_ignored` | [x] |
| E18 | `main`/`printf` | **stdout descriptor closed** at exec (`>&-`) → writes fail `EBADF` | discarded; **exits 0** | `e18_stdout_closed_fd` | [x] |
| E19 | `main`/`printf` | **reader of stdout disappears** (pipe closed early) → `SIGPIPE` with the inherited default disposition | process is **killed by signal 13** (shell status `141`), not a clean exit | `e19_sigpipe_kills_writer` | [x] |
| E20 | `main` | **`scanf` return value discarded**: a partial/failed parse is not detected, so `foo` runs anyway with the surviving defaults | `foo` always called exactly once; exit status always `0` (absent a signal) | `e20_return_value_discarded_exit_zero` | [x] |
| E21 | `main` | **`int main()` takes no parameters**: extra `argv` entries are never inspected | argv ignored entirely; output depends only on stdin; exit 0 | `e21_argv_ignored` | [x] |
| E22 | `main`/`scanf` | **unbounded, never-matching stdin** (`/dev/zero`): the first `%d` hits a non-space, non-digit byte immediately and fails; the stream is *not* drained | returns `0` promptly; `x=y=0`; no output; exit 0 (must **not** hang or buffer the stream) | `e22_unbounded_stdin_not_drained` | [x] |
| E23 | `main`/`scanf` | **conversion stops mid-stream, trailing input never consumed**: `"5 6 7 8 ..."`, `"5 6junk"` | third and later tokens ignored; `foo(5,6)`; exit 0 | `e23_extra_trailing_input_ignored` | [x] |

## Notes on FFI-style "out-of-range enum" inputs

The C code declares no `enum` and exposes no FFI entry point (`foo` is `static`,
the program has no exported functions — see `SYMBOLS.md`), so there is no enum
whose integer representation could be out of range. The corresponding class of
"a value with no valid variant crosses the boundary" is covered by the integer
rows E9–E12: values one step past every documented boundary (`INT_MAX+1`,
`INT_MIN-1`, `UINT_MAX+1`, `LONG_MAX+1`, `LONG_MIN-1`) fed across the real
boundary this program has — its stdin.

## Results

All 23 rows have a passing error-path differential test, plus four extra generic
boundary tests (absent/empty inputs, oversized inputs, every possible leading
byte `0x00`–`0xff`, every possible separator byte `0x00`–`0xff`):

```
$ cargo test --test errors
running 27 tests
test result: ok. 27 passed; 0 failed
```

Each row asserts the *same* rejection from both artifacts — identical stdout,
identical stderr, and the identical exit status or fatal signal — not merely that
"both failed somehow".  Rows whose rejection has an observable signature also pin
the C's own output via `assert_same_expecting`, so a row cannot silently stop
triggering (for example E9 asserts the C prints exactly `"loop\ny\n"`, which is
only true if the saturating conversion really produced `-1`).

Two rows found genuine divergences in the translation, both since fixed:

* **E19 (`SIGPIPE`)** — the Rust runtime installs `SIG_IGN` for `SIGPIPE` before
  `main`, so the Rust program exited 0 where the C died from signal 13 (status
  141).  `src/main.rs` now restores the default disposition.
* **E22 (unbounded stdin)** — the translation slurped stdin with `read_to_end`,
  so `/dev/zero` took seconds and gigabytes of memory where the C exits in 4 ms,
  and a shared seekable stdin was left drained 8192 bytes deep instead of at the
  exact byte the last conversion needed.  `src/main.rs` now reads fd 0 on demand
  and gives back the unused tail with `lseek`, exactly as glibc's stream cleanup
  does.
