# CONFIGS.md — configuration surface table (Phase A, verified in Phase B)

## Build-time configuration axes (enumerated, not guessed)

**Cargo features** — `Cargo.toml` `[features]` contains only `default = []`.
There are no optional features and no optional dependencies, so the complete
set of valid feature combinations is:

| # | combination | cargo invocation |
|---|-------------|------------------|
| F1 | default (empty) | `cargo test --offline` |
| F2 | no-default-features (empty) | `cargo test --offline --no-default-features` |

Both resolve to the identical, single code configuration; there is no
`#[cfg(feature = ...)]` anywhere in `src/`, so no module needs feature gating.
Both are still run explicitly (see `run_all.sh`).

**CMake / C preprocessor options** — `c_src/CMakeLists.txt` contains no
`option()`, no `if()`, no `target_compile_definitions` and no
`CMAKE_BUILD_TYPE` default; `c_src/src/main.c` contains no `#if`/`#ifdef`/
`#ifndef`. The C library therefore has exactly one configuration. The reference
C `.so` is built with CMake's default (unoptimised) flags; the C optimisation
level is additionally swept as row C1 below purely as a robustness check.

## Runtime configuration / input-shape axes (derived from the C branches)

The public surface is the 5 exported functions. The C code branches on exactly
two runtime conditions — `line != NULL` in `printLine` and `x` in `main` — and
its output depends on the `int` value passed to `printIntLine` and on the byte
shape of stdin consumed by `scanf("%d", &x)`. Every row below is a combination
of those axes that the C treats differently. All rows are exercised through
`dlopen`/`dlsym` on **both** `.so` files (never by calling Rust directly), with
many randomized inputs per row from a fixed seed (`SEED = 0x5EED_1234_ABCD_9876`).

### `printLine(const char *)` — lowest-level entry point

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `printLine` | `line == NULL` (guard false) | [x] |
| 2 | `printLine` | empty string `""` (length 0, guard true) | [x] |
| 3 | `printLine` | single-byte strings: every byte value `0x01..=0xFF` (255 cases; `0x00` is the terminator) | [x] |
| 4 | `printLine` | random printable-ASCII strings, lengths 1..64 | [x] |
| 5 | `printLine` | random arbitrary non-NUL byte strings (non-UTF-8 included), lengths 1..64 | [x] |
| 6 | `printLine` | strings containing embedded newlines / `\r` / `\t` / `\v` / `\f` (stdio line-buffer interaction) | [x] |
| 7 | `printLine` | strings containing `printf` conversion specifiers (`%s`, `%d`, `%n`, `%%`) — must be emitted verbatim | [x] |
| 8 | `printLine` | long strings crossing the stdio block-buffer size: lengths 4095, 4096, 4097, 8192, 65536 | [x] |
| 9 | `printLine` | repeated back-to-back calls (many strings in one process; buffer/state leakage) | [x] |

### `printIntLine(int)` — lowest-level entry point

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 10 | `printIntLine` | `0` | [x] |
| 11 | `printIntLine` | small positive / negative (`1`, `-1`, `9`, `-9`, `10`, `-10`) | [x] |
| 12 | `printIntLine` | range endpoints `INT_MAX`, `INT_MAX-1`, `INT_MIN`, `INT_MIN+1` | [x] |
| 13 | `printIntLine` | every power-of-two boundary `±2^k`, `±(2^k−1)`, `±(2^k+1)` for k=0..31 (digit-count and sign transitions) | [x] |
| 14 | `printIntLine` | uniformly random full-range `i32` (many samples) | [x] |
| 15 | `printIntLine` | repeated back-to-back calls (interleaved with `printLine`) | [x] |

### `bad()` / `good()` — the two allocation paths

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 16 | `bad` | single call: `alloca(10)` under-allocated, 40-byte copy loop, prints `data[0]` | [x] |
| 17 | `good` | single call: `alloca(10*sizeof(int))` correctly sized, prints `data[0]` | [x] |
| 18 | `bad`, `good` | many repeated calls in one process (stack-reuse / corruption accumulation) | [x] |
| 19 | `bad`, `good` | interleaved `bad`/`good`/`printIntLine`/`printLine` call sequences (randomized order, 200 ops) | [x] |

### `main()` — the composed pipeline (scanf → branch → alloca → print)

Driven through `dlsym("main")` in a fresh child process per input (so the
stdin/stdout state is virgin, exactly as for a real program start), and
additionally through the two compiled *executables*.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 20 | `main` | stdin empty (immediate EOF) → `scanf` EOF → `x==0` → `bad()` | [x] |
| 21 | `main` | stdin whitespace only, each of `' '`, `'\t'`, `'\n'`, `'\v'`, `'\f'`, `'\r'` and random mixes → EOF after skip → `bad()` | [x] |
| 22 | `main` | leading whitespace then a non-zero number (`"\n\t 7"`) → `x!=0` → `good()` | [x] |
| 23 | `main` | bare `"0"`, `"-0"`, `"+0"`, `"000"` → `x==0` → `bad()` | [x] |
| 24 | `main` | bare non-zero digits, no sign (`"1"`, `"42"`, random 1..9 digits) → `good()` | [x] |
| 25 | `main` | explicit `'+'` / `'-'` sign then non-zero digits → `good()` | [x] |
| 26 | `main` | sign with no digits (`"-"`, `"+"`, `"-x"`, `"+ 5"`) → matching failure → `bad()` | [x] |
| 27 | `main` | leading zeros then non-zero (`"0007"`) → `good()`; and `"0x10"` (`%d` is base 10, stops at `x`) → `bad()` | [x] |
| 28 | `main` | digits followed by trailing garbage (`"5abc"`, `"12,34"`, `"7\n\n"`) → `good()` | [x] |
| 29 | `main` | garbage first (`"abc"`, `".5"`, `"e1"`, random non-numeric) → matching failure → `bad()` | [x] |
| 30 | `main` | `int` boundaries as text: `2147483647`, `2147483648`, `-2147483648`, `-2147483649`, `4294967295`, `4294967296` (the last truncates to `0` → `bad()`) | [x] |
| 31 | `main` | `long` boundaries as text: `9223372036854775807`, `9223372036854775808`, `-9223372036854775808`, `-9223372036854775809` (`strtol` saturation then truncation) | [x] |
| 32 | `main` | very long digit runs (30, 100, 400 digits; all-9s and random digits) — saturation path | [x] |
| 33 | `main` | multiple numbers on the line (`"3 4 5"`) — only the first conversion is performed | [x] |
| 34 | `main` | with and without a trailing newline; input containing an embedded NUL byte before the digits | [x] |
| 35 | `main` | fully randomized byte-string fuzz (512 inputs, lengths 0..24, bytes drawn from a digit/sign/space/garbage alphabet) | [x] |
| 36 | `main` | randomized *decimal* fuzz: uniformly random `i64` and `i128` rendered as text (256 inputs) | [x] |
| 37 | `main` (executables) | the compiled `c_src/build/driver` vs `target/<profile>/driver` over every input of rows 20–36, comparing stdout, stderr and exit status | [x] |

### Process-termination surface (executables) — where the Rust *runtime* differs

The C program's wait status is part of its observable behaviour, and the Rust
runtime's defaults are not C's. These rows compare stdout **and** the exit
code/terminating signal.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 38 | `main` (executables) | stdout is a pipe whose **reader has closed** — the C process is killed by `SIGPIPE` (wait status signal 13); Rust `std` installs `SIGPIPE = SIG_IGN` before `main`, so `src/main.rs` must restore `SIG_DFL`. Five stdin shapes. | [x] |
| 39 | `main` (executables) | stdout is `/dev/full` — every write fails with `ENOSPC`; neither implementation checks `printf`'s return value, so both must still exit 0 | [x] |
| 40 | `main` (executables) | stdout is `/dev/null` — output discarded, exit 0 | [x] |
| 41 | `main` (executables) | stdin descriptor shapes: `/dev/null` (immediate EOF), `/dev/zero` (endless NUL bytes → match failure), **closed** fd 0 (`read` fails `EBADF`), write-only fd as stdin — all must reach `bad()` and print `0\n` | [x] |

> **Divergence found and fixed by row 38.** Before the fix the C binary exited
> with signal 13 while the Rust binary exited 0 on a broken stdout pipe. See
> `restore_default_sigpipe()` in `src/main.rs`. The `#[no_mangle] main` wrapper in
> `src/lib.rs` deliberately does *not* do this: the C `.so`'s `main` does not
> touch the signal disposition either, so the shared objects must both inherit
> whatever the host process configured.

### C build-flag sweep (robustness, not a C configuration option)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| C1 | all 5 | C `.so` rebuilt at `-O0`, `-O1`, `-O2`, `-Os`; Rust `.so` in `debug` and `release` — all pairings must agree | [x] |

### Cargo feature sweep

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| F1 | all 5 | rows 1–41 + C1 under `--features default` (empty) | [x] |
| F2 | all 5 | rows 1–41 + C1 under `--no-default-features` | [x] |

Both combinations are additionally run in the `dev` **and** `release` profiles
by `./run_all.sh`, which also re-derives the feature powerset from `Cargo.toml`
rather than hard-coding it.
