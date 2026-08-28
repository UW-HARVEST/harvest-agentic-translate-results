# Differential verification of the C → Rust translation

Ground truth: `c_src/src/main.c` + `c_src/src/lib.c`, built with CMake.
Both programs are run as subprocesses on identical stdin; **stdout, stderr and
exit status** are compared byte for byte.

```
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
# Rust
cd translation && cargo build --release                                 # -> translation/target/release/driver
# Differential suite
cd translation && cargo test
```

Scale of the suite: **42,574 differential input cases** (85,148 process runs)
across 20 test functions in `tests/differential.rs`. Nothing is `#[ignore]`d,
skipped or disabled.

The suite was run to completion against three builds of the C program, all
passing: the default CMake build (`-O0`), a `Release` build (`-O3`), and a
`--coverage` build. Set `C_DRIVER=/path/to/driver` to point the suite at a
specific C build.

---

## Mismatches found and fixed

### 1. Broken stdout: Rust aborted with `SIGABRT` where C dies from `SIGPIPE`

**Symptom.** With stdout connected to a pipe whose reader has already closed:

| | exit | stderr |
|---|---|---|
| C | killed by signal 13 (`SIGPIPE`) | *(empty)* |
| Rust (before fix) | killed by signal 6 (`SIGABRT`) | `thread 'main' panicked ... failed printing to stdout: Broken pipe (os error 32)` |

Both the exit status *and* stderr differed.

**Cause.** Two independent things, both inherited from Rust's standard library
rather than from the translated logic:

1. Rust's runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, so a write to
   a reader-less pipe returns `EPIPE` instead of killing the process. A C
   program runs with the default disposition and is killed by the signal.
2. `print!` / `eprint!` **panic** when the underlying write fails. C's
   `printf` / `fprintf` just return a negative value, which `main.c` discards.

**Fix** (`src/main.rs`):

* Added `restore_default_sigpipe()`, called first thing in `main`, which resets
  `SIGPIPE` to `SIG_DFL` via the libc `signal()` symbol (declared directly, no
  new crate dependency; `#[cfg(unix)]`-gated).
* Replaced `print!("{}\n", result)` with `let _ = write!(io::stdout(), ...)`
  and the three `eprint!` calls with `let _ = write!(io::stderr(), ...)`, so a
  failed write is discarded exactly as C discards `printf`'s return value.

**Regression test.** `broken_stdout_matches_c` in `tests/differential.rs`
(helper `common::check_broken_stdout`, which builds a pipe with `pipe(2)` and
immediately `close(2)`s the read end). Covers all four operations, the invalid
operation, the empty-decision-string path, and the three stderr-only error
paths.

### 2. Broken stderr: same divergence on the error paths

**Symptom.** With stderr connected to a reader-less pipe and stdin empty (so
`fgets` returns NULL and `main` does `fprintf(stderr, ...)`), C was killed by
`SIGPIPE` while Rust aborted with a panic.

**Cause.** Identical root cause as #1 — `eprint!` panicking plus `SIGPIPE`
being ignored.

**Fix.** Covered by the same change.

**Regression test.** `broken_stderr_matches_c`.

### 3. `fgets` did not return NULL after a mid-line read error (latent)

**Status:** latent divergence found by reading the C standard, **not** observed
as a failing case — it needs a stdin that errors *after* delivering some bytes,
which a pipe or regular file never does.

**Cause.** C7.21.7.2: `fgets` returns a null pointer if end-of-file is
encountered before any character is read **and also if a read error occurs**,
in the latter case even when characters were already stored. The Rust
`fgets` emulation returned `Some(buf)` whenever `buf` was non-empty, so a
mid-line `read` error would have produced a parsed line where C would have
printed `Error reading ...` and exited 1.

**Fix** (`src/main.rs`): `fgets` now tracks `read_error` and returns `None` if
a read error occurred, regardless of how many bytes were already buffered.

### 4. Intermittent failure caused by the *test harness*, not the translation

**Symptom.** One `cargo test --release` run reported `19 passed; 1 failed`
while the surrounding 14 runs were green — a flaky broken-pipe test.

**Cause.** A bug in `tests/common/mod.rs`, not in the Rust program. The
broken-pipe helper built its reader-less pipe with `pipe(2)` and then
`close(2)`d the read end. Those descriptors do **not** have `FD_CLOEXEC`, and
Rust's `std::process::Command` does not close unrelated descriptors in the
child. So if another test thread happened to `fork` in the window between
`pipe` and `close`, that child inherited the **read end** and held the pipe
open for its lifetime — during which writing to the pipe does not raise
`SIGPIPE` at all. Because the harness runs C and Rust through two separate
pipes, the window could hit one side and not the other, producing a spurious
mismatch.

Measured with a standalone reproduction (`8` threads spawning continuously,
`400` trials): the unguarded pattern failed to get `SIGPIPE` in **4/400** runs
for the C binary and **1/400** for the Rust binary. With the fix: **0/400**
for both.

**Fix** (`tests/common/mod.rs`): added a `SPAWN_LOCK: RwLock<()>`. Every
ordinary spawn takes the shared lock (`spawn_guarded`), and the broken-pipe
helper holds the exclusive lock across `pipe` + `close` + `spawn`, so no fork
can occur while the read end exists. The two near-duplicate broken-stdout /
broken-stderr helpers were also merged into one `run_with_broken_stream`.

While fixing this, `check_file_stdin` was moved off `std::env::temp_dir()` onto
`CARGO_TARGET_TMPDIR`, which is inside the crate's own `target/` directory and
therefore always readable and writable.

**Verification.** 8 consecutive `cargo test --release` runs and 6 consecutive
`cargo test` runs, all green.

---

## Behaviours that were checked and already matched

These are recorded because they are the places a translation of this program is
most likely to go wrong; all of them were correct in the Rust before testing.

* **`fgets`, not `scanf`.** Reading stops at `\n` and never crosses it. A
  first line longer than 1023 bytes is *truncated* and the remainder becomes
  the parameter line, then the decision line. Verified for lengths
  1020–1025, 1030 and 2050 (`overlong_lines_spill_into_the_next_read`).
* **`MAX_INPUT_SIZE` truncation.** Each `fgets` stores at most 1023 bytes, so
  an over-long decision line arrives with **no** trailing newline and `main`
  strips nothing, leaving `len == 1023`. Verified for lengths 0–40,
  1015–1030, 2047–2049 and 4096, with and without a final newline
  (`decision_line_length_boundaries`).
* **Order of validation.** `length == 0` is checked *before* the `operation`
  switch, so a blank decision line yields `-1` even for an invalid operation
  (`"99\n0\n\n"` → `-1`, not `-3`). Verified in `main_read_paths`.
* **Embedded NUL bytes.** `fgets` stores them but `strlen`/`atoi` stop at the
  first one, so a NUL truncates whichever line it lands in — e.g.
  `"2\0" + "0\n"` parses as operation `2`, and `"yy\0nn\n"` becomes a
  *2-character* decision string (so operation 0 returns `-2`). Verified in
  `embedded_nul`.
* **`atoi` == `(int) strtol(s, NULL, 10)`.** Leading whitespace (the full
  C-locale `isspace` set including `\v` and `\f`), optional sign, trailing
  junk ignored, no hex, and — crucially — **saturation then truncation**:
  values past `LONG_MAX`/`LONG_MIN` saturate and are then cast to `int`, so
  `"9223372036854775808"` → `-1` and `"-9223372036854775808"` → `0`, while
  `"4294967298"` truncates to `2`. Verified for 32 numeric forms in `atoi`.
* **Newline stripping producing length 0.** A decision line of just `"\n"`
  gives `len == 0` → `-1`.
* **`\r` is data, not a terminator.** `"yyy\r\n"` is a *4*-character decision
  string whose last element parses false, so operation 2 returns `203` and
  operation 3 returns `25`. `\r`-only terminators make the whole stream one
  line (`exotic_whitespace_and_line_terminators`).
* **`parse_bool` defaults to false.** Anything other than `y`/`Y`/`n`/`N` is
  false, including bytes ≥ 0x80 (where C's `char` is signed and Rust's is
  not — equality against `'y'`/`'n'` is unaffected). All 254 usable byte
  values tested as a one-character decision string against all four
  operations.
* **`configure_flags` clamps at 32.** `count = min(length, 32)`, so 33, 40 and
  70 all-`y` characters return `1032`, not `1033`/`1040`/`1070`. The
  `count - 1` comparison is `int` vs `size_t` (the `int` is converted to
  unsigned); `count` is never 0 here because `length == 0` already returned
  `-1`, so it cannot wrap.
* **`validate_sequence` aliases its input buffer.** The C code does
  `bool *bools = (bool*)sequence;` and overwrites byte `i` immediately after
  reading it. Since `sizeof(bool) == sizeof(char) == 1` this is element-wise
  identical to the owned `Vec<bool>` the Rust uses. To be sure the aliasing
  was not being exploited by the optimiser, the suite was run against both an
  unoptimised (`-O0`) and an `-O3` C build: **identical results on all 42,574
  cases in both**.
* **Signed/unsigned comparisons in `validate_sequence`.** `transitions` is
  `int` and `len` is `size_t`, so `transitions == len - 1`,
  `transitions < len / 3`, `transitions > len / 2` and `transitions > len - 3`
  all promote to unsigned; the Rust casts to `usize` in exactly those places
  and uses `i32` for the `transitions < 3` comparison, which is `int` vs `int`.
* **Exhaustive logic tables.** All 8 read/write/execute combinations for
  operation 0; all 8 condition combinations × all four logic operators plus
  out-of-range operators for operation 1; every y/n sequence up to length 12
  for operations 2 and 3.
* **Command-line arguments are ignored** (`main` is `int main(void)`).
* **stdin as a regular file, `/dev/null`, a closed fd, and 1 MiB of input**
  all behave identically; the program reads at most three lines and exits 0.

---

## Unreachable C code (confirmed by coverage, not by guessing)

The suite was re-run against a `--coverage` build of the C program. Result:

```
main.c   Lines executed 100.00% of 19    Branches taken at least once 100.00% of 10
lib.c    Lines executed  98.80% of 166   Branches taken at least once  93.12% of 218
```

Every line and branch outcome that is **reachable through the executable's
interface** is exercised. The shortfall is entirely dead code:

| `lib.c` | Why it cannot be reached |
|---|---|
| L52 `decision_string == NULL` | `main` always passes `input_buffer`, never NULL. |
| L141 `permission_value != 6` | Inside `read && write && !execute`, so `permission_value` is always `4 + 2 == 6`. |
| **L215 `return 90`** | Reached only if XOR is true; XOR true ⇒ exactly one or all three conditions true, which the four preceding `if`s cover exhaustively. |
| **L230 `return 100`** | Reached only if NAND is true; NAND true ⇒ at least one condition false, so one of the preceding `if`s always fires. |
| L249 `i < 32` false | `count = min(length, 32)`, so `i < count` fails first. |
| L265 / L272 loop exhaustion | Guarded by `special_count == 1` / `== count - 1`, so the loop always returns from inside. |
| L319 `len == 0` | `process_decisions` already returned `-1` for `length == 0`. |
| **L372 `return 40`** | Needs `transitions < 3` with `len > 10`. Rule 3 caps runs at 3 equal values, so `len ≥ 11` ⇒ at least `⌈11/3⌉ = 4` runs ⇒ `transitions ≥ 3`. |

The Rust mirrors this structure line for line, so the same statements are
equally dead there — no behaviour depends on them.
