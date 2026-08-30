# Differential verification log — `c_src/src/main.c` vs `translation/`

## What the program does

```c
void driver(int x) { register int y = 2*x; y += 300; printf("%d\n", y); }
int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

The entire observable behavior is: read one `%d` from stdin, print `2*x + 300`
followed by `\n`, exit 0. Nothing is ever written to stderr, and the exit status
is always 0. Every input class is therefore a question about `scanf("%d")` or
about `int` arithmetic.

## How it was verified

- **C binary:** `c_src/build/driver` (`cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`).
  No `CMAKE_BUILD_TYPE` is set by `CMakeLists.txt`, so the C is built
  unoptimized — the signed-overflow UB in `2*x + 300` is compiled as plain
  two's-complement wrapping.
- **Rust binary:** `translation/target/release/driver` (`cd translation && cargo build --release`).
- **Harness:** `translation/tests/differential.rs` spawns both executables as
  subprocesses, writes the same bytes to each one's stdin, and asserts
  byte-identical stdout, byte-identical stderr, and an identical exit status
  (including death-by-signal, compared via `ExitStatusExt::signal`). The Rust
  code is never loaded as a library.
- **Coverage:** 12 tests, ~800 distinct inputs, including a deterministic
  randomized sweep (fixed seed) over the int boundaries, random digit strings up
  to 60 digits, and random raw byte garbage from a `scanf`-relevant alphabet.

## Mismatches found

**None.** Every input tried produced identical stdout, stderr and exit status.
The `translation/src/main.rs` handed to me already modeled each of the risky
behaviors below correctly; no change to the Rust source was required.

To confirm the suite is not vacuously passing, a negative control was run: the
Rust `y += 300` was temporarily changed to `y += 301`, and **all 12 tests
failed**. The change was then reverted (`grep` confirms `wrapping_add(300)`), and
all 12 tests pass again.

## Behaviors that were audited as likely mismatch sources

These are the places a naive translation *would* have diverged. Each was
probed against the C binary and confirmed to agree.

| # | Behavior | C (glibc) result | Why a naive port breaks |
|---|---|---|---|
| 1 | **Failed conversion leaves `x` untouched** — `scanf` return value is ignored by the C | `abc`, `-`, `+`, `- 5`, `--5`, `.5`, `\x00`, `\xff`, empty stdin → `300` | A port using `read_line().parse().unwrap()` would panic (exit 101, output on stderr); one using `unwrap_or(...)` with a non-zero default would print the wrong number. The C leaves `x` at its initializer `0`. |
| 2 | **`scanf` skips leading whitespace across newlines** (unlike `fgets`) | `"\n   \n\t\n  -13\n"` → `274` | A line-oriented reader (`read_line`) stops at the first `\n` and sees an empty line, yielding `300` instead of `274`. |
| 3 | **Conversion stops at the first non-digit; no prefixes** | `0x10` → `300`, `3.9` → `306`, `1e5` → `302`, `1_000` → `302`, `5z` → `310` | `str::parse::<i32>()` rejects the whole token instead of consuming a prefix, so it would fall back to `x = 0`. |
| 4 | **Out-of-`int` values saturate then truncate.** glibc's `%d` accumulates the digits and converts with `strtol`, which saturates at `LONG_MAX`/`LONG_MIN`; the `long` is then stored through an `int *`, i.e. truncated to 32 bits. | `9223372036854775808` → `LONG_MAX` → truncate to `-1` → **`298`**; `-9223372036854775809` → `LONG_MIN` → truncate to `0` → **`300`**; 400 nines → `298` | A port that saturates directly to `i32::MAX` would print `2*2147483647+300` (= `298` by luck) but would print `300` vs the C's `298` for the negative-overflow / long-digit-run cases, and a port that wrapped mod 2^64 would disagree on almost all of them. Note the asymmetry: positive overflow → `298`, negative overflow → `300`. |
| 5 | **In-`int`-range-but-not-in-`i32` values truncate, not saturate** | `2147483648` → `-2147483648` → `300`; `4294967296` → `0` → `300`; `-4294967296` → `0` → `300` | These fit in a `long`, so no saturation happens — only the 32-bit truncating store. Saturating here would give the wrong answer. |
| 6 | **Leading zeros do not count toward overflow** | `0000000000000000009223372036854775808` behaves as `9223372036854775808` → `298`; 500 zeros then `1` → `302` | A digit-count-based overflow heuristic would misfire. |
| 7 | **Signed overflow of `2*x + 300` wraps** | `INT_MAX` → `298`, `INT_MIN` → `300`, `2^30` → `-2147483348` | Rust's `*`/`+` panic on overflow in debug builds and are unspecified-but-wrapping in release; the translation uses `wrapping_mul`/`wrapping_add` so debug and release agree with the C. |
| 8 | **Only the first item is consumed; the rest of stdin is ignored** | `3 4` → `306`, `1 2 3 ... 10` → `302` | A loop-over-all-input port would print several lines. |
| 9 | **Output format is exactly `%d\n`** | one line, trailing newline, no padding, no extra flush output | `println!("{}", y)` happens to match; `{:>N}` or a missing newline would not. |
| 10 | **`argv` is ignored** by `int main()` | `driver 999 --help` behaves identically to no args | A port using `std::env::args()` for input would diverge. |

## Residual, deliberately-not-emulated difference

If **stdout is closed or is a broken pipe**, the C `printf` fails and glibc may
raise `SIGPIPE`, whereas Rust's runtime ignores `SIGPIPE` and the translation
discards the write error (`let _ = write!(...)`). Both were measured with stdout
closed (`>&-`) and both exited `0` with no output, so no divergence is
observable here on this platform; it is recorded only because it is the one
place where the two runtimes' signal dispositions differ in principle. This is
not reachable through stdin-driven grading.

## Final state

- `c_src/` unmodified (only the sanctioned `c_src/build/` output directory added).
- `cargo build --release`: clean, no warnings, no errors.
- `cargo test`: **12 passed, 0 failed, 0 ignored**. No test is disabled,
  skipped, or `#[ignore]`d.
