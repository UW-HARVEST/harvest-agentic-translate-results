# ERRORS.md — differential verification log

Ground truth: `c_src/src/main.c`, built via `c_src/CMakeLists.txt`.
Subject: `translation/src/main.rs`, built with `cargo build --release`.

Run commands recorded in Phase A:

- C: `c_src/build/driver [ARG...]`
- Rust: `translation/target/release/driver [ARG...]`
  (the test harness spawns `CARGO_BIN_EXE_driver`, i.e. `target/debug/driver`
  under `cargo test`; both profiles were checked)

Tests live in `translation/tests/differential.rs`. Every case compares stdout
bytes, stderr bytes and exit status (with signal death distinguished from a
normal exit).

## Mismatches found

**None.** Across every input class enumerated below, the Rust program produced
byte-identical stdout, byte-identical stderr and the same exit status as the C
program. No change to `translation/src/main.rs` was required, and nothing in
`c_src/` was touched.

Because "no mismatch" is only meaningful if the harness can detect one, the
suite was mutation-checked: two deliberate defects were injected into
`src/main.rs` and then reverted.

| Injected defect | Detected by | Observed difference |
| --- | --- | --- |
| `ExitCode::from(1)` → `ExitCode::from(0)` on the `argc != 2` path | `argc_zero_extra_args`, `argc_two_extra_args`, `argc_many_extra_args` | stdout identical, exit status 1 (C) vs 0 (Rust) — exactly the failure mode a stdout-only test would miss |
| `i.wrapping_mul(stride)` → `i.saturating_mul(stride)` | `int_boundaries`, `long_to_int_truncation`, `running_sum_overflow`, `stride_multiplication_overflow`, `sweep_of_many_values` | e.g. argv `268435456`: C emits `1073741824` / `-805306368` for the last two lines, saturating Rust emits `1073741823` / `-1073741826` |

## C branches enumerated, and the input class covering each

The C program has exactly three outcomes.

1. `if (argc != 2)` → prints
   `Error: should only be a single (integer) argument!` **to stdout** (not
   stderr) and returns 1.
   Covered by: no arguments, two arguments, eight arguments.
2. `if (end == argv[1])` → `strtol` consumed nothing, so it reset `end` to the
   start of the string; prints
   `Error: first argument must be an integer!` **to stdout** and returns 1.
   Covered by: empty string, `abc`, whitespace-only (` `, `\t`, `\n`,
   `" \t\r\n\v\f"`), a sign with no digits (`-`, `+`, `--5`, `+-3`, `- 5`),
   leading punctuation (`.5`, `e10`, `x10`, `/1`, `:1`), non-ASCII first byte
   (`é1`, `∞`), and raw invalid UTF-8 first byte (`\xff5`, `\xc3\x28`, `\x80`).
3. The `for (int i = 0; i < 10; i++)` loop, printing the running static total
   with `printf("%d\n", ...)` and returning 0.
   Covered by all remaining cases below.

## Semantic traps checked explicitly

These are the places a translation is most likely to drift. Each was confirmed
equal rather than assumed.

- **Errors go to stdout, not stderr.** `main.c` uses `printf` on both error
  paths. Asserting stderr is empty for both programs pins this down.
- **`strtol` accepts leading whitespace and a sign.** `"  12"`, `"\t-7"`,
  `"\n3"`, `" \t\r\n\v\f42"`, `"+5"`, `"-0"` all parse. Note the C locale is
  never changed (`setlocale` is not called), so `isspace` is the C-locale set.
- **Trailing garbage is not an error.** `end` advances past the digits, so
  `12abc`, `0x10` (base 10: parses `0`, stops at `x`), `3.9`, `7e3`, `1,000`,
  `5\n`, `5 ` and `5\xff` all take the happy path with the parsed prefix.
- **Base 10, so leading zeros are not octal.** `012` is 12, not 10; `-08` is
  valid, not an error.
- **`long` → `int` truncation.** `int stride = strtol(...)` discards the high
  32 bits: `2147483648` becomes `INT_MIN`, `4294967296` becomes `0`,
  `4294967297` becomes `1`.
- **`strtol` range saturation.** Out-of-range input clamps to `LONG_MAX` /
  `LONG_MIN` (glibc), which then truncates to `-1` / `0` respectively.
  `errno` is never inspected by the C code, so saturation is the only
  observable effect. Checked at `±9223372036854775807/8/9`, `2^64`, a
  32-digit literal and a 400-digit literal.
- **Signed `int` overflow, twice over.** Both `i * stride` and the accumulation
  `sum += update` are `int` arithmetic and wrap on this target. The running sum
  is `45 * stride`, so it overflows far earlier than `i * stride` does —
  `47721859` vs `47721860` straddles that boundary, `238609294` vs `238609295`
  straddles the multiplication boundary.
- **Static-variable lifetime.** `static int sum` persists across the ten calls
  within one process and starts at 0 in each new process; every case is a fresh
  subprocess, so both semantics are exercised.
- **Non-UTF-8 argv.** The Rust side reads `args_os` and works on raw bytes, so
  arguments that are not valid UTF-8 reach `strtol` unchanged instead of being
  lossily replaced. Checked with bytes injected before and after the digits.
- **`printf("%d\n", ...)` formatting.** No padding, no precision, one trailing
  newline per line, exactly ten lines. Byte comparison covers this.

## Not a divergence, but worth noting

C dies on `SIGPIPE`; Rust ignores it by default. This is unobservable for this
program: it writes at most ten short lines, which fit in a pipe buffer, so
neither binary ever sees `EPIPE`. Verified with `driver 1 | head -1` (both exit
0) and with stdout closed outright (`>&-`, both exit 0, since the C code ignores
`printf`'s return value).

## Phase D gate

- both programs build with no errors — yes
- every enumerated input matches on stdout, stderr and exit status — yes
- `cargo test` passes: 24 tests, 0 failures
- no test is disabled, skipped or `#[ignore]`d
- `c_src/` unmodified (cmake writes only into the untracked `c_src/build/`)
