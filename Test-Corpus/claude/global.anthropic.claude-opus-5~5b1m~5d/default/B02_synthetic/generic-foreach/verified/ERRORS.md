# Differential verification of the Rust translation against `c_src`

Both programs are driven as subprocesses over identical stdin, and **stdout,
stderr and exit status (exit code *and* terminating signal)** are compared.
Tests live in `tests/differential.rs`.

- C program: `cmake -S c_src -B c_src/build && cmake --build c_src/build`
  → `c_src/build/driver`
- Rust program: `cd translation && cargo build --release`
  → `translation/target/release/driver`

The test harness reuses `c_src/build/driver` when it exists and otherwise
configures CMake out of tree into `translation/target/c_build`, so nothing in
`c_src/` is ever written to.

---

## Mismatches found

### 1. SIGPIPE disposition: C was killed by signal 13, Rust exited 0

**Status: fixed.**

| | stdout reader disappears |
|---|---|
| C (`c_src/build/driver`) | terminated by `SIGPIPE`, status = *killed by signal 13* |
| Rust (before fix) | ran to completion, exit code `0` |

**Cause.** The Rust standard library installs `SIG_IGN` for `SIGPIPE` during
runtime start-up, before `main` is entered. A C program starts with the
*default* disposition, so the first `printf`/flush into a pipe whose reader has
gone kills it outright. In the Rust version the write instead returned `EPIPE`,
which `src/cstdio.rs`'s `p!` macro deliberately discards (mirroring the fact
that the C code never checks `printf`'s return value), so the loop carried on
and the process exited `0`.

Reproduction, before the fix:

```
C: returncode=-13      # killed by signal 13
R: returncode=0
```

**Fix.** `reset_sigpipe()` in `src/main.rs` restores `SIG_DFL` for `SIGPIPE` as
the very first statement of `main`, via a direct `extern "C" { fn signal(...) }`
declaration (no new dependency):

```rust
fn main() {
    reset_sigpipe();
    ...
}
```

Covered by `dead_stdout_reader_terminates_both_the_same_way`. Removing the
`reset_sigpipe()` call makes that test fail on 5 out of 5 runs.

---

## Test-harness defect found and fixed (not a translation bug)

### 2. The SIGPIPE test was itself racy — it failed in *both* directions

The first version of that test fed `6\n6\n6\n7\n` (~18 KB of output) and then
closed the read end of the stdout pipe. 18 KB fits inside the default 64 KiB
pipe buffer, so whichever process won the scheduling race could write
everything and exit `0` before the reader was closed. Over 25 full-suite runs it
failed 7 times, sometimes reporting `C (None, Some(13)) vs Rust (Some(0), None)`
and sometimes the exact opposite — proof the test, not the translation, was at
fault.

The test now feeds `"6\n".repeat(500)` (~3 MB of output). That is far more than
a pipe can hold, so the child cannot finish ahead of the close: it either hits
`EPIPE` immediately or blocks on a full pipe and is then woken by the closed
read end. 30 consecutive full-suite runs, 0 failures.

---

## Behaviour that was checked and already matched

Verified with ~340 inputs (hand-enumerated classes, a fixed-seed sweep of 250
inputs in the suite, plus 3 000 ad-hoc random inputs during investigation —
0 mismatches).

**Reading (`fgets` does *not* read across newlines):** `char input[256]` takes at
most 255 bytes, so a longer line is split across successive iterations and each
piece is parsed on its own. Lines of 254 / 255 / 256 / 257 / 300 / 10 001
bytes, 255 bytes of pure whitespace (which yields `Invalid input` and leaves the
rest of the line for the next read), and a final line with no trailing newline
all agree.

**`sscanf(input, "%d", &choice)` order and semantics:** the parse is attempted
*before* the `switch`, so `Invalid input` (no conversion) and `Invalid choice`
(converted but outside 1–7) are distinct paths and both were exercised. Prefix
conversions (`3abc`, `5.9`, `1 2`, `0x10` → `0`, `007`, `+4`, leading spaces,
tabs, `\v`, `\f`) match; non-conversions (empty line, whitespace only, `\r\n`,
`+`, `-`, `++7`, `.5`, `e5`, a leading NUL, `0xff 0xfe`, U+FF17) match.

**Integer overflow / truncation exactly as C does it:** glibc converts through a
`long` that *saturates* at `LONG_MAX`/`LONG_MIN`, then stores it into an `int`,
which *truncates*. Both effects are observable and both agree:

| input | value stored in `choice` | effect |
|---|---|---|
| `4294967297` (2³²+1) | `1` | really runs demo 1 |
| `4294967302` (2³²+6) | `6` | really runs all demos |
| `-4294967295` | `1` | really runs demo 1 |
| `18446744073709551617` (2⁶⁴+1) | `-1` (saturated, not wrapped) | `Invalid choice` |
| `9223372036854775808`, twenty `9`s, forty digits | `-1` | `Invalid choice` |

**`printf` formatting:** `%.2f` and `%.1f` (prices, averages, temperatures),
`%zu` for `size_t`, `%lld` for the `long long` product, the `°` in demo 2, the
box-drawing banner (40 `═` per line) and every trailing newline are byte
identical.

**Fixed-width string fields:** `strncpy` into `char[64]`/`char[32]` followed by a
forced NUL, and `%s` printing that stops at the NUL, are reproduced by
`strncpy_truncate` / `cstr`.

**Control flow:** all seven `case`s plus `default`; `case 7`'s early `return 0`;
loop exit via `fgets` returning NULL (empty input, EOF after a demo, and
`/dev/null` stdin); `argv` ignored because `main` is declared `(void)`.

**Streams:** neither program writes a single byte to stderr, and both exit `0`
on every non-signal path. Two guard tests (`neither_program_writes_to_stderr`,
`output_is_not_trivially_empty`) keep the stdout/stderr comparisons from passing
vacuously.

**Unreachable C code** — `calculate_inventory_stats`'s `size == 0`,
`calculate_order_stats`'s `size == 0`, `find_items_by_category`'s
`found == 0` and null guards, and `find_expensive_items` (never called) — cannot
be reached from `main`, since the demos always populate fixed data. They are
translated faithfully but no input can exercise them.

---

## Suite validation (mutation testing)

To confirm the tests are not vacuous, five deliberate defects were injected into
the Rust source one at a time; every one was caught, and all were reverted:

| injected defect | caught by |
|---|---|
| `reset_sigpipe()` call removed | `dead_stdout_reader_terminates_both_the_same_way` |
| `"Invalid input"` → `"Invalid Input"` | 5 tests |
| `print_item` price `%.2f` → `%.3f` | 5 tests |
| `fgets` limit `size - 1` → `size` | 3 tests |
| `sscanf` saturating the `int` instead of truncating | `overflow_and_truncation_of_choice` |

## Current state

`cargo test` (debug and `--release`): **19 passed, 0 failed, 0 ignored.** No test
is disabled, skipped or `#[ignore]`d. `cargo build --release` produces no
warnings. `c_src/` is unmodified.
