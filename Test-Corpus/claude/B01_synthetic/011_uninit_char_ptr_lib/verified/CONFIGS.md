# CONFIGS.md — Configuration-surface table (Phase B)

Mirror of `ERRORS.md` for **valid** inputs. Axes derived mechanically from what
the C code actually branches on, not from what looks important.

## Axes the C code distinguishes

**Runtime options/flags.** The library has exactly one: the `int useGood`
parameter of `driver`. `grep` finds exactly two conditionals in the whole
translation unit, and these are the only branch points:

| source | branch | toggles |
|--------|--------|---------|
| `driver.c:51` | `if (useGood)` | `good()` (non-zero) vs `bad()` (zero) |
| `driver.c:30` | `if (line != NULL)` | emit `puts(line)` vs emit nothing |

There is no init/teardown, no global or static state, no handle/context object,
no mode enum, no byte-order or width option, and no `#ifdef`-selected behaviour.

**Input shapes.** The only data input in the API is `printLine`'s `const char *`.
The shapes the code distinguishes are: NULL vs non-NULL; and, for non-NULL,
byte length (0 / 1 / many / past-stdio-buffer) and byte content (the content
matters because C reaches `puts` while Rust reaches `printf("%s\n", …)`, so any
divergence in format-string handling or byte transparency shows up here).

**Full set of public entry points.** All four exported symbols are covered, and
the tests drive the **lowest-level** one (`printLine`) directly rather than only
the `driver` convenience wrapper:

| entry point | level | header-declared |
|-------------|-------|-----------------|
| `printLine(const char*)` | lowest — does the actual I/O | no (exported anyway) |
| `bad(void)`  | mid — calls `printLine` with an uninitialized pointer | no (exported anyway) |
| `good(void)` | mid — calls `printLine("string")` | no (exported anyway) |
| `driver(int)`| top — dispatches to `good`/`bad` | **yes** |

## Configuration rows

Cross-product of {entry point} × {option value} × {input shape}, pruned to the
combinations the C actually treats differently. Every row is driven through both
`.so` files and compared byte-for-byte on `stdout`. Rows marked *(random)* use
many property-style generated inputs with a fixed seed (`SEED = 0x5EED_1234`)
rather than one hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | non-NULL, empty string `""` (length 0 boundary) | [x] |
| 2 | `printLine` | non-NULL, length 1, all 255 possible non-NUL byte values (exhaustive) | [x] |
| 3 | `printLine` | non-NULL, random ASCII printable, lengths 2..=64 *(random, 400 cases)* | [x] |
| 4 | `printLine` | non-NULL, payload of `printf` conversion specifiers: `%s %d %n %% %p %1000d` interleaved with text *(random, 200 cases)* | [x] |
| 5 | `printLine` | non-NULL, control bytes `\n \r \t \x01 \x1b \x7f` embedded *(random, 200 cases)* | [x] |
| 6 | `printLine` | non-NULL, arbitrary high/non-UTF-8 bytes `0x80..=0xFF` *(random, 200 cases)* | [x] |
| 7 | `printLine` | non-NULL, arbitrary bytes `0x01..=0xFF` full range, lengths 1..=256 *(random, 600 cases)* | [x] |
| 8 | `printLine` | non-NULL, long strings crossing the stdio buffer: 1 KiB, 4 KiB, 8 KiB, 64 KiB, 256 KiB *(random content)* | [x] |
| 9 | `printLine` | non-NULL, length exactly at power-of-two/`BUFSIZ` boundaries: 511/512/513, 1023/1024/1025, 4095/4096/4097, 8191/8192/8193 | [x] |
| 10 | `printLine` | NULL (the one rejection branch, valid-path side) | [x] |
| 11 | `good` | no options; single call | [x] |
| 12 | `good` | no options; repeated 50× (buffering / idempotence) | [x] |
| 13 | `driver` | `useGood = 1` → `good()` branch | [x] |
| 14 | `driver` | `useGood` = random non-zero positive `i32` *(random, 300 cases)* | [x] |
| 15 | `driver` | `useGood` = random non-zero **negative** `i32`, incl. `-1` and `INT_MIN` *(random, 300 cases)* | [x] |
| 16 | `driver` | `useGood` = `INT_MAX`, `INT_MIN`, `1`, `-1`, `2`, `0x8000_0000u32 as i32` (boundary values) | [x] |
| 17 | `driver` | `useGood = 0` → `bad()` branch — UB at `-O0`, compared against `-O2` C where it is well-defined (see `ERRORS.md` §UB) | [x] |
| 18 | `bad` | direct low-level call — UB at `-O0`, compared against `-O2` C (see `ERRORS.md` §UB) | [x] |
| 19 | composed pipeline | randomized interleaving of `printLine`/`good`/`driver(nonzero)` in one captured stream — verifies output **ordering** and shared-`stdout` buffering of the composed sequence, which per-call tests cannot see *(random, 100 sequences of 20 calls)* | [x] |
| 20 | composed pipeline | `printLine` immediately after `good()`/`driver(1)` — checks no residual state or interleaving artifacts between entry points | [x] |

## Notes

* Row 2 is exhaustive over all 255 non-NUL single bytes, since that is cheap and
  is the sharpest test of `puts` vs `printf("%s\n")` byte transparency.
* Rows 8 and 9 exist because C reaches `puts` while Rust reaches `printf`; the
  two use different internal fast paths in glibc for long strings and at buffer
  boundaries, so lengths spanning `BUFSIZ` are where a divergence would appear.
* Row 19 is the "real consumer" row required by Phase B: state is set up, the
  option is applied, and the whole operation runs end to end, with several entry
  points composed into one output stream.
* All rows are run under the single valid feature combination (there is no
  `[features]` section — see `SYMBOLS.md`).

## Results

All 20 rows pass, under every configuration (see `./verify.sh`):

| configuration | result |
|---------------|--------|
| `cargo test --no-default-features` (the only feature combination) | 45/45 pass |
| `cargo test` (default features) | 45/45 pass |
| `cargo test --all-features` | 45/45 pass |
| `cargo test --release` (optimized cdylib, `panic = "abort"` profile) | 45/45 pass |

Test files:

| file | tests | covers |
|------|-------|--------|
| `tests/configs.rs` | 18 | Phase B — CONFIGS.md rows 1–16, 19, 20 |
| `tests/errors.rs` | 9 | Phase C — ERRORS.md rows 1–7, 9, 10 |
| `tests/ub_bad.rs` | 8 | CONFIGS.md rows 17–18 / ERRORS.md row 8, plus the `-O0`/`-O2` codegen pin |
| `tests/symbols.rs` | 3 | Phase A/D — `nm -D` symbol parity, enforced automatically |
| `tests/harness_selftest.rs` | 7 | negative controls: proves the suite actually detects divergence |

### Why the results are trustworthy (negative controls)

A differential suite that always passes proves nothing, so
`tests/harness_selftest.rs` builds deliberately **wrong** variants of the C
source (into `target/`; `c_src/` is never touched) and asserts the very same
comparisons used above report a divergence:

| mutation | caught by |
|----------|-----------|
| `printf("%s\n", line)` → `printf("%s", line)` (dropped newline) | `printLine` comparison |
| `if (useGood)` → `if (useGood > 0)` | only the **negative** boundary values of row 16 / ERRORS row 9 — confirming those rows are load-bearing |
| `NULL` path given an observable side effect | ERRORS row 1 |
| `good()`/`bad()` branches swapped | row 13 |

Plus two tests asserting the capture mechanism returns real, distinguishable
bytes and is not polluted by surrounding output.

### Two harness bugs the suite found in itself

1. **fd-1 redirection is process-global.** With libtest's default parallel
   runner, one test's capture window swallowed another test's output, producing a
   spurious "divergence" in row 19 (C appeared to emit 6 extra `string\n`).
   Fixed by `.cargo/config.toml` (`RUST_TEST_THREADS = "1"`) plus a global mutex
   held across each capture, and by draining both Rust's `std` stdout buffer and
   libc's `stdout` buffer before stealing the descriptor.
2. **The `-O0` C `bad()` can segfault.** An early test asserted it "returns
   normally"; that assumption held in the debug test binary but SIGSEGV'd in the
   release one, because the stack residue differed. It is now run in a forked
   child via `run_isolated`, which characterises either outcome without killing
   the suite — see `ERRORS.md` §UB.
