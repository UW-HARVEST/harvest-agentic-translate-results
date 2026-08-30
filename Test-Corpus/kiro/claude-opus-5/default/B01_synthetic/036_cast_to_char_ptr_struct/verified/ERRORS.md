# ERRORS.md — differential verification log

Verification of `translation/` (Rust) against `c_src/` (C, ground truth).

Both programs are compared by **execution**: same bytes on stdin, then stdout,
stderr and exit status are diffed. Harness: `translation/tests/differential.rs`.

- C build: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → executable `c_src/build/driver`
- Rust build: `cd translation && cargo build --release`
  → executable `translation/target/release/driver`
- Run command for both: `./driver < input` (no arguments, no environment
  dependence)

## Result

**No mismatches were found.** Every input class enumerated below produced
byte-identical stdout, byte-identical stderr (always empty) and exit status 0
from both programs.

Coverage actually executed:

- 24 integration tests in `translation/tests/differential.rs`, all passing in
  both the `dev` and `release` profiles, none `#[ignore]`d.
- Exhaustive sweeps inside those tests: all 256 single-byte inputs, and all
  1024 two-byte inputs of the form `{'-', '+', ' ', '\n'} × 0x00..=0xff`.
- 6000 randomized differential cases (random byte soup over a
  digit/sign/whitespace/punctuation alphabet, plus random 1–80-bit integers with
  random sign) — 0 diffs.
- `c_src/src/main.c` is unmodified (only the out-of-tree `c_src/build/`
  directory was created, as the build instructions require).

## What the C program does, and every branch in it

```c
typedef struct { int floors; int bedrooms; double bathrooms; } house_t;
static void print_hex(unsigned char *p, int len);   // 16 iterations, "%02x", then "\n"
void driver(int floors);                            // house = {0}; then 3 field stores
int main(void) { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

There is **no** `if`, no early `return`, no length check and no null check in
the C source. `print_hex` always runs exactly `sizeof(house_t) == 16`
iterations, `driver` is unconditional, and `main` always returns 0. Therefore
every behavioural branch lives in the `scanf("%d", &x)` conversion, and the
input classes are the states that conversion can end in. There is no error path
that writes to stderr or returns non-zero — the C program exits 0 for all
inputs, including garbage.

Enumerated input classes (each has a test):

| Class | Example input | C behaviour replicated |
| --- | --- | --- |
| EOF before any non-whitespace | `""`, `" "`, `"\n"`, `"\t\t\t"` | `scanf` returns `EOF`, **`x` is not assigned** and keeps its initial `0` |
| Whitespace skip, incl. across newlines | `"\n\n\n42\n"`, `"\v\f\r 9"` | `%d` skips all `isspace` bytes (` \t\n\v\f\r`) and crosses newlines |
| Matching failure — no digit | `"abc"`, `".5"`, `"e5"`, `",1"`, `"\0"` | `scanf` returns `0`, `x` stays `0` |
| Matching failure — sign then non-digit | `"-"`, `"+"`, `"--5"`, `"- 5"` | `scanf` returns `0`, `x` stays `0` |
| Single successful item | `"1"`, `"0"`, `"3"`, `"-1"`, `"+5"` | value assigned |
| Conversion stops at first non-digit | `"5abc"`, `"12.7"`, `"0x10"`, `"5 9"`, `"1\n2"` | only the leading digit run is consumed; `0x10` is decimal `0`, not hex |
| Leading zeros | `"007"`, `"0000000009"` | decimal, **not** octal (`%d` is base 10) |
| 32-bit extremes | `INT_MAX`, `INT_MIN` | exact |
| Fits `long`, overflows `int` | `2147483648`, `4294967295`, `2^33`, `3000000000`, `-2147483649` | glibc converts with `strtol` then stores `(int) num.l` → **low 32 bits, wrapping** |
| Overflows `long` | `9223372036854775808`, `2^64`, 26 nines, 5000 nines | `strtol` **saturates** at `LONG_MAX`/`LONG_MIN`, then truncates to `int` (`LONG_MAX` → `-1`, `LONG_MIN` → `0`) |
| Embedded NUL | `"\0 5"`, `"5\0"` | NUL is neither space nor digit: matching failure, or terminates a digit run |
| Arbitrary binary | all 256 byte values, high-bit bytes, invalid UTF-8 | matching failure; Rust must not panic on non-UTF-8 |
| Very long tokens / prefixes | 5000-digit runs, 9000-space prefix, 1 MB trailing input | identical; unread remainder is simply never consumed |

## Translation details that had to match, and were checked

These are the places where a plausible translation *would* have diverged. Each
was verified against the C binary rather than assumed.

1. **`x` is not written on a failed conversion.** `int x = 0; scanf("%d", &x);`
   leaves `x == 0` when `scanf` returns `0` or `EOF`. `scanf_i32` in
   `src/main.rs` returns `Option<i32>` and `main` only assigns on `Some`, so the
   `0` survives. A translation that used `unwrap_or(0)` would coincidentally
   agree here, but one that errored out or exited non-zero on bad input would
   not — the C program exits **0** on `"abc"`.

2. **Overflow is saturate-then-truncate, not wrap.** glibc's `%d` runs
   `strtol` and stores `(int) num.l`. So `"9223372036854775808"` (> `LONG_MAX`)
   clamps to `LONG_MAX` and prints `ffffffff` (i.e. `-1`), *not* `0` as a pure
   128-bit wrap would give. `scanf_i32` reproduces this by accumulating in `i64`
   with `checked_*`, latching to `i64::MIN`/`i64::MAX` on overflow, then `as
   i32`. Confirmed on `LONG_MAX±1`, `LONG_MIN±1`, `2^64`, and 5000-nine inputs.

3. **Negative values accumulate negatively.** Accumulating the magnitude and
   negating at the end would make `"-9223372036854775808"` overflow one step too
   early. `scanf_i32` uses `checked_sub` while negative, matching `strtol`.

4. **Struct layout and endianness.** The program hex-dumps the raw bytes of
   `house_t`, so the output *is* the ABI layout: `floors` at offset 0,
   `bedrooms` at offset 4, `bathrooms` (8-byte aligned) at offset 8,
   `sizeof == 16`, little-endian. `house_t house = {0}` zero-fills the whole
   object. Rust models this as a zeroed `[u8; 16]` with explicit
   `to_le_bytes()` stores rather than relying on `#[repr(C)]` plus
   `transmute`, which keeps the dump deterministic. Pinned by
   `a_known_good_output_shape`: input `1` ⇒
   `01000000030000000000000000000040\n` (`2.0f64` = `0x4000000000000000`).
   Note this layout is x86-64 System V; the tests compare against the C binary
   built on the same host, so any layout assumption is validated rather than
   trusted.

5. **`isspace` set.** `%d` skips exactly ` `, `\t`, `\n`, `\v`, `\f`, `\r` in
   the C locale — `is_c_space` lists the same six. Verified per-byte by
   `c_every_single_byte_input` and
   `c_sign_or_space_followed_by_every_byte`.

6. **Output formatting.** `printf("%02x", p[i])` × 16 with no separators,
   followed by a single `printf("\n")`, and nothing else — no leading label, no
   trailing space. Rust uses `{:02x}` and one `writeln!`. Byte-compared, so a
   stray newline or uppercase hex would fail.

7. **Write errors are ignored, exit code stays 0.** The C code never checks
   `printf`'s return value, so a failing stdout still yields `return 0`. The
   Rust code discards the `write!`/`flush` results for the same reason.
   Checked manually with stdout redirected to `/dev/full`: both exit `0`.

8. **Exit status is always 0.** `main` has a single `return 0` and no path can
   abort. The Rust `main` returns `()` (status 0) and, notably, must not panic
   on any input — non-UTF-8 stdin is read as raw bytes, never as a `String`,
   which is why `c_binary_input` passes.

## Not covered

- Non-x86-64 / big-endian hosts: the byte image would differ from the hardcoded
  little-endian offsets, but so would the C program's, and both are compared on
  the same host, so a real divergence there is untested rather than known-good.
- A C library other than glibc: the saturate-then-truncate overflow behaviour of
  `%d` is glibc's (`strtol` + `(int)` cast). Under a library that behaves
  differently on overflow, the overflow cases would need re-checking.
