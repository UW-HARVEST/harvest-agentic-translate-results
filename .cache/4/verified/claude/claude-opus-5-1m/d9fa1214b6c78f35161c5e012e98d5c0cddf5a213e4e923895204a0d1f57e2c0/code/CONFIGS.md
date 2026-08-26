# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

## Mechanical derivation of the axes

`c_src/CMakeLists.txt` has **no** `option()`, no `target_compile_definitions`,
no `CMAKE_BUILD_TYPE` handling — it is three lines that compile one file. The C
source contains **no** `#if`/`#ifdef`, no global mode/flag variable, no
`switch`, and exactly one `if`-like construct (the `for` guard `i < x`).
Consequently there is **no compile-time or runtime option surface**: the only
axes are the input shapes and the environment the two entry points observe.

`Cargo.toml` declares `[features] default = []` and no optional features, so the
feature-combination cross-product has exactly **one** member (`<none>`, which is
identical to the default). See `check_all_features.sh`.

### Axis list (derived from the source, not guessed)

| axis | values the C code actually distinguishes | why (source evidence) |
|------|------------------------------------------|-----------------------|
| A. entry point | `driver` (lowest level, the `int`-taking worker) · `main` (the `.so`'s other exported symbol: `scanf` + `driver`) · the `driver` **executable** (`add_executable`) | `nm -D` shows both symbols; CMake builds an exe |
| B. `x` sign | `x <= 0` (loop body skipped) · `x > 0` | the guard `i < x` |
| C. `x` magnitude / decimal width | 1 · 2–9 · ≥10 · ≥100 · ≥1000 · ≥10⁴ · ≥10⁵ | `printf("%d")` field width of `i` grows with `i` |
| D. `j` decimal width, independent of `i` (`j == 2*i`) | `j` crosses 10 at `i=5`, 100 at `i=50`, 1000 at `i=500`, 10⁴ at `i=5000`, 10⁵ at `i=50000` → the two printed columns have *different* widths | `printf("%d %d", i, j)` |
| E. total output size vs the two stdio buffer sizes | < 4096 B · straddling 4096 B (glibc `stdout` block) · straddling 8192 B (Rust `BufWriter`) · ≫ both | C buffers with `FILE*`, Rust with `BufWriter<StdoutLock>`; a mismatch shows up as truncated/duplicated bytes |
| F. stdout kind | regular file · pipe | glibc picks the buffering mode from `fstat`; Rust does not |
| G. calls per process | 1 · N in a row (buffer/lock state reuse) | `driver` is re-entrant from a `.so`; both impls keep global stream state |
| H. stdin leading whitespace (for `main`) | none · `' '` · `'\t'` · `'\n'` · `'\v'` (0x0b) · `'\f'` (0x0c) · `'\r'` · a mix / several | `%d` skips `isspace()` in the "C" locale |
| I. stdin sign | none · `'+'` · `'-'` | `strtol` sign handling |
| J. stdin leading zeros | none · `"0"` · `"000…0"` + digits | `strtol` accumulation |
| K. stdin trailing bytes after the digits | immediate EOF · `'\n'` · other whitespace · non-digit garbage · a **second** number (must be ignored) | one `%d` directive only |
| L. stdin value range | 0 · 1 · small · > `INT_MAX` (truncated) · > `LONG_MAX` (saturated) | `strtol` → `(int)` store |
| M. stdin source kind | pipe · regular file | glibc read-ahead differs, must stay unobservable |

## Configuration table

Every row is exercised against **both** `.so`s through `libloading` (and, for
the `exe` rows, against both linked executables), with many randomized inputs
per row driven by a fixed-seed xorshift PRNG.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` via `.so` | `x = 0` → empty output (axes B, E<4096) | [x] |
| 2 | `driver` via `.so` | `x = 1` → exactly one line `"0 0\n"` (C) | [x] |
| 3 | `driver` via `.so` | `x = 2..9`, exhaustive → single-digit `i`, `j` crosses 10 at `i=5` (C, D) | [x] |
| 4 | `driver` via `.so` | `x = 10` → `i` reaches two digits, `j` reaches two digits (C, D) | [x] |
| 5 | `driver` via `.so` | `x = 51` → `j` crosses 100 while `i` is still two digits (D) | [x] |
| 6 | `driver` via `.so` | `x = 100`, `101` → `i` crosses 100 (C) | [x] |
| 7 | `driver` via `.so` | `x = 501`, `1000`, `1001` → `j` crosses 1000, `i` crosses 1000 (C, D) | [x] |
| 8 | `driver` via `.so` | `x` chosen so the byte count straddles 4096: every `x` whose output length ∈ [4000, 4200] (E, F=file) | [x] |
| 9 | `driver` via `.so` | `x` chosen so the byte count straddles 8192 (E) | [x] |
| 10 | `driver` via `.so` | `x = 5000`, `5001`, `10000`, `10001` → `j` crosses 10⁴, `i` crosses 10⁴; output ≫ both buffers (C, D, E) | [x] |
| 11 | `driver` via `.so` | `x = 50000`, `50001`, `100000` → `j` crosses 10⁵ (D, E ≫) | [x] |
| 12 | `driver` via `.so` | randomized `x ∈ [0, 200000]`, 200 samples, fixed seed (B–E) | [x] |
| 13 | `driver` via `.so` | randomized `x ∈ [i32::MIN, 0]`, 200 samples (B) | [x] |
| 14 | `driver` via `.so` | stdout is a **pipe** instead of a regular file, `x ∈ {0,1,7,1000}` (F) | [x] |
| 15 | `driver` via `.so` | **N=50 consecutive calls** in one process with mixed `x` (incl. 0s), output concatenated (G) | [x] |
| 16 | `main` via `.so` | stdin `"7"`, no trailing newline, EOF right after the digits (H none, K EOF) | [x] |
| 17 | `main` via `.so` | stdin `"7\n"` (K `'\n'`) | [x] |
| 18 | `main` via `.so` | stdin with each single leading whitespace byte in `{' ','\t','\n','\v','\f','\r'}` before the number (H) | [x] |
| 19 | `main` via `.so` | stdin with a randomized *mixture* of ≥2 leading whitespace bytes (H) | [x] |
| 20 | `main` via `.so` | stdin `"+n"` explicit plus sign (I) | [x] |
| 21 | `main` via `.so` | stdin `"-n"` → negative `x` → empty output (I, B) | [x] |
| 22 | `main` via `.so` | stdin `"0"`, `"-0"`, `"+0"` (I, J, L=0) | [x] |
| 23 | `main` via `.so` | stdin with 1–40 leading zeros before the digits (J) | [x] |
| 24 | `main` via `.so` | stdin digits immediately followed by non-digit garbage (`"5abc"`, `"5.9"`, `"5-3"`) — only the first number converts (K) | [x] |
| 25 | `main` via `.so` | stdin with a **second number** after whitespace (`"5 9"`, `"5\n9\n"`) — second must be ignored (K) | [x] |
| 26 | `main` via `.so` | stdin value > `INT_MAX` but in `long`, truncating to a **small positive** `int` (e.g. `"4294967300"` → 4) (L) | [x] |
| 27 | `main` via `.so` | randomized decimal strings for `x ∈ [0, 20000]` with randomized whitespace/sign/zero-padding, 200 samples (H–L) | [x] |
| 28 | `main` via `.so` | stdin is a **pipe** rather than a regular file, same shapes as rows 16/17/25 (M) | [x] |
| 29 | `driver` exe | end-to-end: both linked executables, stdin from a pipe, stdout to a pipe, randomized inputs from rows 16–27 (A=exe, F=pipe, M=pipe) | [x] |
| 30 | `driver` exe | end-to-end: stdin from a **regular file**, stdout to a **regular file** (A=exe, F=file, M=file) | [x] |
| 31 | `driver` exe | end-to-end: exit status must be 0 for every valid input (A=exe) | [x] |
| 32 | `driver` exe | large output (`x = 200000`, ≈2.3 MB) to a pipe — many `write(2)`s, buffer-boundary stress (E, F) | [x] |

Legend: `[x]` = the row's differential test passes for both implementations
across all of its randomized inputs (see `tests/`).

## Phase B results

`tests/phase_b_configs.rs` reports

```
running 32 tests
...
test result: ok. 32 passed; 0 failed
```

so every row in the table above passes, across all of its randomized inputs
(fixed PRNG seeds, listed per test, ~1 000 distinct inputs in total).

### How the rows are driven

* **Rows 1–15** call the `driver` symbol *directly* through `dlopen`/`dlsym` on
  both shared objects — the lowest-level entry point, not a convenience wrapper.
  File descriptor 1 is redirected to a capture file (or to a pipe for row 14),
  the symbol is invoked, `fflush(NULL)` drains glibc's `FILE*` buffer, and the
  bytes are compared. Row 15 issues 50 calls in one process so that any
  difference in retained buffer/lock state shows up.
* **Rows 16–28** call the `main` symbol through `dlopen`/`dlsym` in a **fresh
  child process per input**, because both implementations keep global buffered
  stdin state — one `scanf` per process is exactly what the C program does, and
  reusing a process would test something the C program never does.
* **Rows 29–32** run the two linked executables end to end, which is the only way
  to observe process-level behaviour (wait status, stdio buffering mode chosen
  from `fstat`, `SIGPIPE`).

### Why the test binaries use `harness = false`

Both implementations write to file descriptor 1, so every comparison must
temporarily redirect fd 1 — process-global state. libtest runs cases on several
threads and writes its own `test … ok` lines to fd 1 while doing so, and those
lines landed *inside* the captured bytes, producing spurious mismatches. The
suites therefore use a tiny sequential harness (`common::run_cases`).

### Configuration axes that are deliberately not rows

* **`x` beyond ≈2·10⁵** — rows 6 and 21 of `ERRORS.md` cover `INT_MAX`-sized
  output on a bounded 64 KiB prefix; enumerating 2³¹ lines is not feasible.
* **`j`'s signed overflow at `i == 2³⁰`** — unreachable in bounded time; it is C
  undefined behaviour and the Rust code uses `wrapping_add`, matching the code
  gcc actually emits.
* **stdout being a terminal** (glibc would then pick line buffering) — not
  reproducible in a batch test without a pty; the pipe and regular-file cases
  (rows 14, 29, 30, 32) cover both buffering modes glibc selects for
  non-terminals, and the Rust side never varies its buffering by fd type.

### Compile-time configuration

`Cargo.toml` declares `[features] default = []` with no optional features, and
`c_src/CMakeLists.txt` defines no options and no preprocessor symbols, so the
feature cross-product has exactly one member. `check_all_features.sh` enumerates
it mechanically and `cargo check --no-default-features --features ""` plus the
default build both succeed. `run_difftests.sh` re-runs symbol parity and both
differential suites for every enumerated combination; the suites additionally
pass in the `release` profile (`cargo test --release`), which is the other
build-time configuration this crate has (`panic = "abort"`).
