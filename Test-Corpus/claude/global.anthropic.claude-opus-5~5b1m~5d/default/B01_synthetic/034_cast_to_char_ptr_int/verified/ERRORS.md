# Differential testing log: `c_src` vs `translation`

The C program in `c_src/src/main.c` is the ground truth. Both programs were
built and run as subprocesses over the same inputs, comparing **stdout**,
**stderr** and **exit status** (including death-by-signal).

* C:    `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver`
* Rust: `cd translation && cargo build --release`
  → `translation/target/release/driver`
* Suite: `cd translation && cargo test` (also verified with `cargo test --release`).
  `tests/differential.rs` builds the C program itself if `c_src/build/driver` is
  absent, and exercises **both** the `cargo test` Rust binary and the
  `--release` artifact when present.

## What the program does

```c
static void print_hex(unsigned char *p, int len) {
    for (int i = 0; i < len; i++) printf("%02x", p[i]);
    printf("\n");
}
void driver(int x) { print_hex((unsigned char *)&x, sizeof(x)); }
int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

Read one decimal integer with `scanf("%d")`, then hex-dump the 4 bytes of that
`int` in memory order (little-endian on x86-64), lowercase, then `\n`. Output is
always exactly 9 bytes; stderr is always empty; exit status is always 0 — except
for the `SIGPIPE` case below.

---

## Mismatches found

### 1. `SIGPIPE`: Rust exited 0 where the C program was killed by signal 13

**Status: FIXED.**

**Symptom.** With stdout connected to a pipe that has no reader:

| | stdout | stderr | wait status |
|---|---|---|---|
| C (expected) | empty | empty | **killed by signal 13** (`SIGPIPE`) |
| Rust (before) | empty | empty | **exited 0** |

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` in its runtime
init *before* `main` runs. A C program does not: it keeps the default
disposition. So when the C program's exit-time `stdout` flush hits a broken
pipe, `write` raises `SIGPIPE` and the process dies from the signal. The Rust
build instead got `EPIPE` back from `write_all`, which the translation was
discarding with `let _ = ...`, and then fell off the end of `main` returning 0.

This is exactly the failure mode the task description warns about: **stdout and
stderr matched perfectly and only the exit status differed.** A stdout-only
comparison would have passed.

**Fix.** `translation/src/main.rs` now restores the default `SIGPIPE`
disposition as the first thing `main` does, via a dependency-free `extern "C"`
declaration of `signal(2)` (`SIGPIPE` = 13, `SIG_DFL` = 0 on Linux/glibc):

```rust
fn main() {
    restore_default_sigpipe();
    ...
}
```

**Regression test.** `dies_from_sigpipe_when_stdout_has_no_reader`. It closes
the read end of the child's stdout pipe *before* feeding stdin, so the child is
still blocked in `read` when the pipe breaks — no race. The test also asserts
the C side really is `(code: None, signal: Some(13))`, so it fails loudly if the
premise ever stops holding rather than silently comparing two zeros.

---

## Behaviours that looked like bugs and were deliberately preserved

These are not defects in the translation; they are C behaviour that had to be
reproduced, and each has a test.

1. **`scanf`'s return value is ignored.** Every conversion failure leaves `x` at
   its initializer `0`, so the program happily prints `00000000` and exits **0**
   for garbage input. There is no error path and no non-zero exit status
   anywhere in this program. Tests: `empty_input`,
   `matching_failure_on_non_digit`, `matching_failure_after_lone_sign`.

2. **`%d` skips whitespace across newlines.** Unlike `fgets`, the number may sit
   several blank lines down; `\n \n \n 123` reads 123. All six C whitespace
   characters (`' ' \t \n \v \f \r`) are skipped. Test:
   `scanf_reads_across_newlines`, `whitespace_only_input`.

3. **A lone sign is a *matching* failure.** `-`, `+`, `-\n`, `--5`, `- 5` all
   store nothing → `00000000`. Test: `matching_failure_after_lone_sign`.

4. **`long`-then-truncate, not saturate.** glibc implements `%d` by running its
   `strtol` machinery into a `long` and then assigning that `long` to the `int`
   object. So values above `INT_MAX` **wrap**:
   `2147483648` → `00000080` (`INT_MIN`), `4294967295` → `ffffffff` (`-1`),
   `4294967296` → `00000000`. The translation accumulates in `i64` and does
   `as i32`. Test: `long_to_int_truncation`, `int_boundaries`.

5. **Saturation only at the `long` boundary.** Past `LONG_MAX`/`LONG_MIN`,
   `strtol` clamps *first* and the clamped value is then truncated, so
   `9223372036854775808` → `LONG_MAX` → `ffffffff`, while
   `-9223372036854775808` → `LONG_MIN` → `00000000`. A 400-digit or 4097-digit
   run of nines gives the same answer as `LONG_MAX`. Test: `long_saturation`,
   `very_long_digit_runs`.

6. **Conversion stops at the first non-digit and the rest of stdin is never
   read.** `0x10` reads `0` (decimal, not hex) → `00000000`; `3.99` reads `3`;
   `5 6` reads only `5`. Test: `stops_at_first_non_digit`.

7. **Leading zeros are decimal, not octal**, and cannot overflow. Test:
   `leading_zeros`.

8. **`%02x` is lowercase and zero-padded**, the loop runs exactly
   `sizeof(int)` == 4 times, and the trailing `printf("\n")` is unconditional.
   Test: `hex_formatting_covers_all_nibbles` pins `1732584193` → `01234567\n`
   and `-271733879` → `89abcdef\n`, covering all 16 nibble values;
   `output_shape_is_always_nine_bytes` pins the length and empty stderr.

---

## Input classes covered (24 tests, ~5000 subprocess pairs)

| Class | Test |
|---|---|
| both binaries build and run | `both_binaries_are_runnable` |
| empty input (EOF, nothing read) | `empty_input` |
| whitespace-only (all 6 C space chars) | `whitespace_only_input` |
| single value, with/without trailing `\n` | `single_value` |
| `+` / `-` / `-0` | `signs` |
| number several lines down | `scanf_reads_across_newlines` |
| trailing junk, two numbers, `0x`, floats | `stops_at_first_non_digit` |
| first char not a digit (incl. NUL, `\xff`) | `matching_failure_on_non_digit` |
| lone sign / sign + non-digit | `matching_failure_after_lone_sign` |
| `INT_MAX` / `INT_MIN` exactly | `int_boundaries` |
| above `int`: wrap via `long`→`int` | `long_to_int_truncation` |
| at/above `long`: `strtol` saturation | `long_saturation` |
| leading zeros | `leading_zeros` |
| 1–4097 digit runs (buffer-growth sizes) | `very_long_digit_runs` |
| all 16 hex nibbles, byte order | `hex_formatting_covers_all_nibbles` |
| exact output length / empty stderr | `output_shape_is_always_nine_bytes` |
| stdin = `/dev/null` | `stdin_from_devnull` |
| extra `argv` (C `main()` takes none) | `extra_argv_is_ignored` |
| stdout = `/dev/null` | `stdout_to_devnull` |
| 1 MiB stdin, junk before/after number | `huge_stdin` |
| **stdout pipe with no reader → SIGPIPE** | `dies_from_sigpipe_when_stdout_has_no_reader` |
| exhaustive length 0/1/2, and length 3 | `exhaustive_short_inputs` |
| 400 seeded pseudo-random byte strings | `randomized_inputs` |
| every decimal near 2^0 … 2^64 (±2, ±sign) | `numeric_sweep` |

## Suite validated by mutation

To confirm the tests actually detect divergence rather than passing vacuously,
three mutations were injected into `translation/src/main.rs` and then reverted:

| Mutation | Result |
|---|---|
| remove the `restore_default_sigpipe()` call | `dies_from_sigpipe_when_stdout_has_no_reader` FAILED (1 of 24) |
| clamp to `i32` range instead of truncating | 5 tests FAILED, incl. `long_to_int_truncation`, `long_saturation` |
| uppercase hex digits (`b'A'`) | 15 tests FAILED, incl. `hex_formatting_covers_all_nibbles` |

All three were reverted and the suite is green again (24 passed, 0 failed,
0 ignored) in both the debug and release profiles.

## Notes

* No test is `#[ignore]`d, skipped or disabled.
* Nothing in `c_src/` was modified; only `c_src/build/` (a generated CMake
  output directory) was created.
* One intentional structural difference: the translation slurps all of stdin
  with `read_to_end` and parses from a buffer, whereas `scanf` reads lazily.
  This is unobservable for this program — it prints once and exits — and was
  checked with 1 MiB inputs (`huge_stdin`) and with unreadable stdin (a closed
  fd 0 and a directory as stdin both behave as EOF in each program, matching).
