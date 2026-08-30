# Differential verification of the C → Rust translation

Ground truth: `c_src/src/main.c`. The Rust program must produce byte-identical
stdout, byte-identical stderr and the same exit status for the same stdin.

Test suite: `tests/differential.rs` (21 tests). It builds `c_src` with cmake if
needed, then runs the C `driver` and every available Rust `driver`
(`CARGO_BIN_EXE_driver`, plus `target/release/driver` when it exists) as
subprocesses and compares stdout, stderr, exit code and terminating signal.
Nothing is linked as a library. No test is `#[ignore]`d, skipped or disabled.

## Commands

| | |
|---|---|
| build C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` |
| run C | `c_src/build/driver < input` |
| build Rust | `cd translation && cargo build --release` |
| run Rust | `translation/target/release/driver < input` |
| test | `cd translation && cargo test` |

## What the C program branches on

`main` contains no `if`; every branch that matters is inside the two libc calls:

* `scanf("%lf", &f)` — leading-whitespace skip (newlines included), EOF/input
  failure, optional sign, `inf`/`infinity` and the partial spellings glibc
  rejects, `nan`, the `0x`/`0X` hex form, the decimal form, `p`/`e` exponents
  with and without digits, matching failure, overflow, underflow, subnormals.
  On **any** failure `f` keeps its initialiser, `0.0`, and the program still
  prints a line and exits 0.
* `printf("%llx %a %.4f\n", u.x, f, f)` — the raw bits read back out of the
  union, `%a` for zero / subnormal / normal / inf / nan with trailing hex-zero
  trimming, and `%.4f` including the exact halfway decimals.

## Mismatches found

Two mismatches were found. Both were exit-status divergences that a
fixed-input stdin/stdout diff cannot expose, which is why they survived
~15 000 fixed-input comparisons.

### 1. Endless stdin: the Rust program hung where the C program exits

* **Symptom** — `yes 1.5 | driver`: the C program prints
  `3ff8000000000000 0x1.8p+0 1.5000` and exits 0 immediately; the Rust program
  never terminated (killed by a 3 s timeout, status 124, no output).
* **Cause** — `main` did `stdin().read_to_end(&mut input)` and only then
  parsed. `scanf` does the opposite: it consumes bytes one at a time, stops at
  the first byte that cannot extend the conversion, and never waits for EOF.
  Against a producer that does not close the pipe, reading to EOF cannot
  return. The same bug also made memory use proportional to the whole stream
  rather than to the token.
* **Fix** — `src/main.rs`: added `struct Input<R: Read>`, a lazily-filled view
  of the stream with random access by absolute index (`at(i)` returns `None`
  only at true end of stream, and pulls another chunk only when asked for an
  index it has not reached). `scan_double`, `scan_hex`, `scan_decimal` and
  `ci_prefix_len` now index through it instead of through a `&[u8]` slurped up
  front. The parsing logic is otherwise unchanged: `i < len && input[i] == X`
  became `input.matches(i, X)`, `i >= len` became `input.at(i).is_none()`, and
  so on.
* **Regression test** — `endless_stdin_terminates_like_the_c_program`. Feeds
  `"1.5\n"` for ever from a helper thread and polls `try_wait` against a 20 s
  deadline. Verified to fail (`did not terminate on an endless stdin`) when the
  `read_to_end` version is put back.

### 2. Closed stdout: the Rust program exited 0 where the C program dies from SIGPIPE

* **Symptom** — with stdout a pipe whose reader has already gone away
  (`printf 1.5 | driver | true`), the C program is killed by `SIGPIPE`
  (shell status 141, signal 13); the Rust program exited 0.
* **Cause** — the Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs,
  so the failing write returns `EPIPE` instead of raising a signal. The
  previous code then discarded that `io::Result` (`let _ = write!(...)`) and
  fell off the end of `main` with status 0. The C program runs with the default
  disposition and is terminated by the signal.
* **Fix** — `src/main.rs`: `restore_default_sigpipe()`, called first in `main`,
  puts `SIGPIPE` back to `SIG_DFL` via a direct `extern "C" fn signal`
  declaration (no new dependency; libc is already linked). Discarding the write
  result is now correct, because the process dies before the result is seen —
  which is exactly what the C program does.
* **Regression test** — `closed_stdout_dies_from_sigpipe_like_the_c_program`.
  Spawns each binary with stdout on a pipe, drops the read end, and compares
  exit code *and* terminating signal. Verified to fail when the
  `restore_default_sigpipe()` call is removed.

## What was checked and matched with no change needed

Roughly 15 000 fixed inputs plus the in-suite sweeps agreed byte for byte, in
both the debug and the release profile. The translation already reproduced
these glibc behaviours, each of which a naive translation gets wrong; they are
listed because they are the places where a future edit is most likely to
regress.

* `inf` needs 3 matching characters, `infinity` needs 8, and **4 through 7 are
  errors**: glibc commits to the long spelling as soon as a 4th matching
  character appears, so `infi`, `infin`, `infini`, `infinit` all leave `f` at
  0.0 and print `0 0x0p+0 0.0000`.
* A bare `0x` prefix is a conversion error, but `0x.` is *not*: it is long
  enough to reach `strtod`, which converts the leading `0`, stops at the `x`,
  and succeeds — so `-0x.` yields **negative** zero
  (`8000000000000000 -0x0p+0 -0.0000`), not a failure.
* `0x1p`, `0x1p+`, `0x1pz` are all `1.0`: `strtod` stops before a `p` that has
  no exponent digits. Likewise `5e`, `5e+`, `1e+-5` are all `5.0`/`1.0`.
* `nan` and `-nan` produce the default quiet NaN, sign included, which `%llx`
  makes visible (`7ff8000000000000` / `fff8000000000000`). A parenthesised
  payload is not consumed and does not change the value.
* `%a` renders subnormals with a `0` leading digit and exponent `-1022`
  (`5e-324` → `0x0.0000000000001p-1022`), zero as `0x0p+0`, and trims trailing
  hex zeros from the mantissa.
* `%.4f` on exact halfway decimals rounds to even (`0.03125` → `0.0312`), and
  renders large values in full (`1e308` is 309 integer digits plus `.0000`).
* Overflow saturates to `inf` and underflow to a *signed* zero: `-1e-400` →
  `-0.0000`, and an exponent that overflows the accumulator itself
  (`0x1p99999999999999999999`) still gives `inf`.
* Whitespace skipping crosses newlines, and every failure path still exits 0
  with empty stderr.

Coverage in the suite: all 256 single bytes, all pairs over the 47 bytes the
scanner reacts to, ~7 000 random byte strings including NUL and non-ASCII,
~3 000 random bit patterns round-tripped as hex floats, ~3 000 random decimal
and ~3 000 random hex literals, ~4 200 mantissas straddling the 53-bit
rounding boundary in the normal and subnormal ranges, and inputs up to 1 MB.

## Suite validation

The suite was mutation-tested to confirm it is not vacuous. Seven deliberate
breakages of `src/main.rs` were each caught:

| mutation | tests that failed |
|---|---|
| `%.4f` → `%.5f` | all 19 comparison tests |
| accept `infi` as `inf` | 3 |
| `0x.` returns failure instead of signed zero | 3 |
| `%a` exponent `+` → space | all 19 |
| `std::process::exit(1)` at the end of `main` (stdout unchanged) | all 19 |
| remove `restore_default_sigpipe()` | `closed_stdout_dies_from_sigpipe…` |
| restore `read_to_end` | `endless_stdin_terminates…` |

The `exit(1)` row is the important one: it confirms exit status is compared and
not just stdout.

## Note on `c_src/`

Nothing under `c_src/` was modified. `c_src/CMakeLists.txt` and
`c_src/src/main.c` are untouched; only the build output directory
`c_src/build/` was created.
