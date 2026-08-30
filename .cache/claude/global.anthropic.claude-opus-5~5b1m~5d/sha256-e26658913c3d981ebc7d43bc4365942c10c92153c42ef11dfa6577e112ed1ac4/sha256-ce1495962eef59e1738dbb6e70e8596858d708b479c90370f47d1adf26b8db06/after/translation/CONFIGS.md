# CONFIGS.md — Phase A configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the C
source and the C header, not from a guess at what "matters".

## Axis enumeration (derived from the source)

**Axis 1 — public entry points.** `include/driver.h` declares only `driver`, but
the built `.so` exports four symbols (see `SYMBOLS.md`). All four are real
public entry points and all four are tested directly — *including the
lowest-level one*, `printLine`, not just the `driver` one-shot wrapper:

| entry point | arity | calls |
|-------------|-------|-------|
| `printLine` | 1 arg (`const char *`) | `puts` (lowered from `printf("%s\n", …)`) |
| `bad`       | 0 args | `printLine` with a fixed literal |
| `good`      | 0 args | `printLine`, then `static helperGood` → `printLine` |
| `driver`    | 0 args | 4 direct `printLine` calls interleaved with `good` and `bad`; 7 lines total |

**Axis 2 — runtime options / modes / flags.** **None.** There is no init
function, no context/handle struct, no setter, no global mutable state, and no
`switch`. The library is stateless: grep finds no `static` variable and no
non-`const` global. Consequently there is no option cross-product to enumerate
— the configuration space is driven entirely by input *shape*.

**Axis 3 — input shapes the code distinguishes.** The single branch
(`if (line != NULL)`) distinguishes null from non-null; `puts` is then
byte-oriented and length-sensitive, so the shapes that can change observable
behaviour are:

- length: empty (0) / one byte / many bytes / long enough to cross libc's
  `BUFSIZ` stdout buffer (≥ 4096, and ≥ 8192 to force multiple flushes)
- byte values: ASCII printable / high bytes ≥ 0x80 (non-UTF-8) / control bytes
  (`\t`, `\r`, `\n`, `\x1b`) / `%` and `\\` (format-specifier lookalikes, which
  must be emitted literally because the C passes them as an *argument* to `%s`)
- pointer shape: start-of-buffer / interior pointer / buffer whose first byte is
  the NUL terminator

**Axis 4 — compile-time configuration.** `Cargo.toml` has no `[features]`
section and the C has no `#ifdef`, so there is exactly one configuration. Rows
below are verified under both the default build and `--no-default-features`
(which are the same build).

## Configuration-surface table

Each row is exercised with **many randomized inputs** from a fixed seed
(`SEED = 0x5EED_1234_ABCD_0001`, deterministic xorshift64* in
`tests/common/mod.rs`), not one hand-picked value, and compared byte-for-byte
between the two `.so`s.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `printLine` | non-null, length 1..=64, random printable ASCII (256 random inputs) | `cfg_01_print_line_short_ascii` | [x] |
| 2 | `printLine` | non-null, length 1..=4096, random **arbitrary non-zero bytes** incl. high/non-UTF-8 and control bytes (256 random inputs) | `cfg_02_print_line_random_bytes` | [x] |
| 3 | `printLine` | non-null, length 4000..=20000 — crosses and exceeds libc `BUFSIZ`, forcing intermediate `stdout` flushes (64 random inputs) | `cfg_03_print_line_crosses_bufsiz` | [x] |
| 4 | `printLine` | non-null, bytes drawn only from the format-lookalike alphabet `% s d n \ " '` (128 random inputs) — pins that `%` is emitted literally | `cfg_04_print_line_format_lookalikes` | [x] |
| 5 | `bad` | no args, no state — fixed single-line output | `cfg_05_bad` | [x] |
| 6 | `good` | no args — two lines, exercises the `static helperGood` call edge | `cfg_06_good` | [x] |
| 7 | `driver` | no args — full end-to-end pipeline, 7 lines, pins ordering of `good`/`bad`/literals | `cfg_07_driver` | [x] |
| 8 | `printLine` | non-null, interior pointer into a larger buffer (offset 1..=32), random bytes (128 random inputs) | `cfg_08_print_line_interior_pointer` | [x] |
| 9 | `printLine`, `bad`, `good`, `driver` | **composed pipeline**: random-length random sequence of calls to all four entry points, driven the way a real consumer drives the library, asserting the whole accumulated stdout stream matches (64 random sequences of up to 32 calls) | `cfg_09_random_call_sequence` | [x] |
| 10 | `printLine` | non-null, single byte, swept over **every** value `0x01..=0xFF` exhaustively (255 inputs) — value-dependent byte handling | `cfg_10_print_line_every_single_byte` | [x] |

Row 9 is the row that per-wrapper tests cannot cover: it interleaves the
low-level `printLine` with the composed `driver`/`good`/`bad` on a shared,
buffered `stdout`, so it catches divergence in flush timing and output ordering
rather than only in the content of one call.

## Verification results

All 10 rows pass across their randomized inputs, under every feature
combination and both profiles:

```
PASS (25 tests)  cargo test <default>              <debug>
PASS (25 tests)  cargo test <default>              --release
PASS (25 tests)  cargo test --no-default-features  <debug>
PASS (25 tests)  cargo test --no-default-features  --release
PASS (25 tests)  cargo test --all-features         <debug>
PASS (25 tests)  cargo test --all-features         --release
```

### The suite was proved capable of failing

Passing tests only mean something if they *can* fail. Nine mutations were
injected into `src/lib.rs`, rebuilt, and run against the C reference; eight
compiled and **all eight were caught**:

| mutation | caught by |
|----------|-----------|
| M1 `printLine`: drop the NULL guard | `err_01`, `err_01b`, `cfg_09` (SIGSEGV vs clean exit) |
| M2 `bad()`: change the literal text | `cfg_05`, `cfg_07`, `cfg_09` |
| M3 `good()`: drop the `helperGood()` call | `cfg_06`, `cfg_07`, `cfg_09` |
| M4 `driver()`: swap `good()`/`bad()` order | `cfg_07`, `cfg_09` |
| M5 `driver()`: drop the final line | `cfg_07`, `cfg_09` |
| M6 `helperGood()`: change the literal | `cfg_06`, `cfg_07` |
| M7 `printLine`: treat `""` like NULL | `err_02` |
| M8 `printLine`: route through Rust `str` (mangles non-UTF-8) | `cfg_02`, `err_04` |
| M9 emit `\r\n` instead of `\n` | did not compile; skipped |

### Independent end-to-end cross-check

Beyond the Rust harness, an external **C** consumer (`dlopen` + `dlsym`, driving
`printLine`/`driver`/`good`/`bad` including the NULL and non-UTF-8 inputs) was
run against both `.so`s with stdout as a pipe, as a regular file, and as a TTY
(i.e. libc line-buffered rather than fully buffered). Output was byte-identical
in all three modes, confirming the match does not depend on the harness's own
buffering assumptions.
