# Differential verification log — `c_src/src/main.c` vs `translation/`

Method: build both executables, feed them identical stdin, and compare stdout
(byte for byte), stderr (byte for byte) and exit status. Driven by
`translation/tests/differential.rs` — the Rust binary is only ever run as a
subprocess, never linked as a library.

Commands:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> translation/target/release/driver
cd translation && cargo test                                            # differential suite
```

## What the program actually does

```c
int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

`driver` fills `house_t { int floors; int bedrooms; double bathrooms; }` with
`{0}`, then `floors = x`, `bedrooms = 3`, `bathrooms = 2.0`, `memcpy`s the
struct into a `char[16]` and hex-dumps all 16 bytes with `%02x` followed by
`"\n"`. On x86-64 SysV there is no padding, so the output is always exactly
33 bytes:

```
<floors little-endian> 03000000 0000000000000040
```

Exit status is always 0 and stderr is always empty — *except* for the SIGPIPE
case below. There are no error-message paths in the C at all; the only
"error path" is `scanf` failing to convert, which stores nothing and leaves
`x == 0`.

---

## Mismatches found

### 1. `SIGPIPE` disposition — Rust exited 0 where the C is killed by signal 13

**Severity: real behavioural divergence in exit status.**

Reproducer (stdout is a pipe whose read end is already closed):

```
$ echo 5 | c_src/build/driver          > >(exec true); echo $?
141
$ echo 5 | translation/target/release/driver > >(exec true); echo $?
0        # <-- before the fix
```

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` in its runtime
init before `main` is entered. Consequently `printf`'s Rust equivalent
(`write_all` to stdout) returned `Err(EPIPE)`, which `print_hex` discards with
`let _ = ...`, and the process ran to completion returning 0. The C program
keeps the default disposition, so the `write` inside `printf` raises `SIGPIPE`
and the process is *killed* by signal 13 — which shells report as status 141
and which `ExitStatus::code()` reports as `None`, not `Some(0)`.

Note this is invisible to any test that only compares stdout: with a closed
stdout there *is* no stdout to compare, and both stderrs are empty. Only the
exit status differs — exactly the failure mode the task description warns
about.

**Fix.** `translation/src/main.rs` now restores the default disposition as the
first statement of `main`:

```rust
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
    unsafe { signal(SIGPIPE, SIG_DFL); }
}
```

**Regression test.** `closed_stdout_pipe_matches_sigpipe_death`. It compares
both `ExitStatus::code()` *and* `ExitStatusExt::signal()`, and avoids the
obvious race by exploiting the fact that the child blocks reading stdin: the
read end of the child's stdout pipe is dropped *before* any stdin is written,
so stdout is guaranteed closed by the time `printf` runs.

---

## Behaviours that were checked and already matched

These are recorded because each one is a plausible way a translation goes
wrong, and each is now pinned by a test rather than left to chance.

| Area | C behaviour | Verified by |
|---|---|---|
| `scanf` skips **all** leading whitespace, newlines included (unlike `fgets`) | `"\n\n\n12"` → 12 | `scanf_reads_across_newlines_and_stops_at_first_conversion` |
| Only one conversion is performed; the rest of stdin is never read | `"12 34"` → 12 | same |
| Matching failure stores nothing, so `x` keeps its initialiser `0` | `"abc"`, `"+"`, `"-"`, `".5"`, `"- 12"` → 0 | `matching_failure_leaves_x_at_zero` |
| EOF before any conversion also leaves `x == 0` | empty stdin, `/dev/null`, whitespace-only | `empty_input`, `stdin_is_empty_device` |
| A lone sign with no digits is a matching failure, not zero-by-parse | `"+"` / `"-"` → `00000000...` | `matching_failure_leaves_x_at_zero` |
| glibc's `%d` accumulates into a `long`, saturating like `strtol`, then stores the **truncated low 32 bits** into the `int` | `"2147483648"` → `00000080` (= `INT_MIN`); `"4294967296"` → `00000000`; `"9223372036854775808"` → `ffffffff` (`LONG_MAX` truncated = -1); `"-99999999999999999999999999"` → `00000000` (`LONG_MIN` truncated = 0) | `integer_overflow_truncation_and_signedness` |
| Saturation is on the *accumulator*, so arbitrarily long digit runs do not wrap repeatedly | 10 000 `9`s → `ffffffff`, not a wrapped value | `maximum_length_digit_runs` |
| Leading zeros are consumed without triggering saturation | 10 000 `0`s then `7` → `07000000` | same |
| `%02x` on `unsigned char` never sign-extends | high bytes in the `floors` image, e.g. `-1` → `ffffffff` | `pinned_output_bytes_match_the_struct_layout` |
| `sizeof(house_t) == 16`, `{0}` zeroes the (nonexistent) padding, `2.0` is `0000000000000040` LE | every case | `both_binaries_run_and_emit_32_hex_digits_plus_newline`, `pinned_output_bytes_...` |
| Exactly one trailing `"\n"`, no separators between hex pairs | 33 bytes of stdout | same |
| Non-UTF-8 / embedded-NUL stdin must not make the Rust reader diverge | `"\0\0 12"` → 0, `"12\0 34"` → 12, 1 KiB of `0xff` → 0 | `binary_and_embedded_nul_input` |
| Non-ASCII digits are not digits | Arabic-Indic `١٢` → 0 | `matching_failure_leaves_x_at_zero` |

## Coverage

Both binaries agree on all of:

- every hand-enumerated case above (~90 explicit inputs)
- every integer in `[-300, 300]`, plus `2^n - 1`, `2^n`, `2^n + 1` and their
  negations for `n = 0..63` (`exhaustive_small_values_and_powers_of_two`)
- 600 deterministic pseudo-random strings over the alphabet `%d` cares about
  (digits, signs, all six `isspace` bytes, letters, `.` `,` `_` `x` `X`, NUL)
- 600 deterministic pseudo-random numeric literals at widths
  1/2/3/5/9/10/11/15/19/20/21/40 with random signs and trailing junk
- 300 deterministic strings of fully arbitrary bytes (`0x00`–`0xff`)

Final state: `cargo test` → 15 passed, 0 failed, 0 ignored. No test is
disabled, skipped or `#[ignore]`d. Nothing in `c_src/` was modified (only the
`c_src/build/` output directory was created, per the build instructions).
