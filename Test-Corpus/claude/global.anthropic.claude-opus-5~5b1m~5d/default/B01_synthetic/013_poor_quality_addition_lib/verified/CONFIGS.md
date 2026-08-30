# CONFIGS.md — Configuration surface table (valid inputs)

## Axis enumeration (mechanically derived from the C source)

### Public entry points

`nm -D` on the C `.so` yields five exported functions. Note that `driver.h` only
declares `driver()` — the "convenience / one-shot wrapper". The four *lower-level*
entry points (`printLine`, `printIntLine`, `bad`, `good`) are non-`static` and
therefore fully callable by any consumer through `dlsym`, so they are all
exercised **directly**, not only via `driver()`.

| level | entry point | parameters |
|-------|-------------|------------|
| 0 (lowest) | `printLine`    | `const char *` |
| 0 (lowest) | `printIntLine` | `int` |
| 1 | `bad`  | none (calls `printIntLine` twice) |
| 1 | `good` | none (calls `printIntLine` twice) |
| 2 (wrapper) | `driver` | none (calls `printLine` ×4, `good`, `bad`) |

### Runtime options / modes / flags

Grep for option state:

```
$ grep -n 'static\|extern\|#ifdef\|#if \|switch\|enum\|struct\|global' c_src/src/driver.c
(no matches)
```

There is **no** runtime option, mode, flag, global variable, struct, enum,
`switch` or `#ifdef` anywhere in the library. The only branch in the entire
library is `printLine`'s `if(line != NULL)`. Therefore the configuration axes
collapse onto **input shape** plus **entry point** plus **call sequencing**
(stdio buffering state), and the ambient axis below.

### Ambient axis the C code is sensitive to

Both libraries write through the *same process-wide* libc `stdout`. The stream's
buffering mode is therefore an axis that changes the observable byte ordering:

* `A_pipe` — `stdout` redirected to a file/pipe ⇒ fully buffered,
* `A_unbuf` — `setvbuf(stdout, NULL, _IONBF, 0)` ⇒ unbuffered,
* `A_line` — `setvbuf(stdout, NULL, _IOLBF, ..)` ⇒ line buffered.

Note: GCC rewrites the C `printf("%s\n", line)` into `puts(line)` while the Rust
translation keeps `printf`. Under a *shared* buffer these must still produce an
identical byte stream in every buffering mode — which is exactly what the
`A_*` rows below pin down.

### Input shapes the code distinguishes

`const char *` (for `printLine`): non-NULL vs NULL (NULL → `ERRORS.md` E1);
length 0 / 1 / many / huge; byte content ASCII / non-ASCII / format-specifier-
looking / embedded control characters.

`int` (for `printIntLine`): sign (negative / zero / positive), decimal digit
count 1..10, and the two extremes `INT_MIN` / `INT_MAX`. The `%d` conversion
branches on sign and on digit count, so all of those are distinct shapes.

## Configuration table

One row per meaningful combination the C actually treats differently. Every row
is driven with **many randomised inputs (fixed seed `0x5EED_1234_ABCD_F00D`)**
except where the row's input space is a singleton.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C01 | `printIntLine` | `int` = `0` (singleton; digit-count 1, sign zero) | [x] |
| C02 | `printIntLine` | `int` ∈ random positive 1-digit … 9-digit values (sign +, digit count 1‥9) | [x] |
| C03 | `printIntLine` | `int` ∈ random positive 10-digit values (`1_000_000_000..=INT_MAX`) | [x] |
| C04 | `printIntLine` | `int` ∈ random negative values, 1‥9 digits (sign −) | [x] |
| C05 | `printIntLine` | `int` ∈ random negative values, 10 digits (`INT_MIN+1..=-1_000_000_000`) | [x] |
| C06 | `printIntLine` | `int` = `INT_MAX` and `INT_MIN` (extremes) | [x] |
| C07 | `printIntLine` | `int` = every `±1` neighbourhood of each power-of-ten digit boundary (`9`,`10`,`99`,`100`,…) and each power-of-two boundary | [x] |
| C08 | `printIntLine` | `int` ∈ 4096 uniformly random `i32` values (full-range sweep) | [x] |
| C09 | `printLine` | non-NULL, length 0 (`""` → pointer to lone `'\0'`) | [x] |
| C10 | `printLine` | non-NULL, length 1, random printable ASCII byte | [x] |
| C11 | `printLine` | non-NULL, random length 2‥64, random printable-ASCII content | [x] |
| C12 | `printLine` | non-NULL, random length 1‥256, random content over the **full** `0x01..=0xFF` byte range (non-ASCII, invalid UTF-8) | [x] |
| C13 | `printLine` | non-NULL, content is format-specifier-looking text (`%s`, `%d`, `%n`, `%%`, `%1000000d`) | [x] |
| C14 | `printLine` | non-NULL, content contains embedded `\n` / `\r` / `\t` (multi-line payload) | [x] |
| C15 | `printLine` | non-NULL, large payloads: lengths 4 KiB, 64 KiB, 1 MiB (crosses the stdio buffer size, forcing internal flushes) | [x] |
| C16 | `bad` | no input (deterministic; asserts the CWE-482 defect is preserved: `0` then `0`) | [x] |
| C17 | `good` | no input (deterministic: `0` then `2`) | [x] |
| C18 | `driver` | no input (full composed pipeline: 4 × `printLine` + `good` + `bad`) | [x] |
| C19 | mixed, low-level | randomised **sequences** of 1‥40 calls drawn from {`printLine`, `printIntLine`, `bad`, `good`, `driver`} with random arguments — exercises the composed pipeline & carried stdio buffer state, which per-function tests cannot see | [x] |
| C20 | `driver` ×N | `driver` called repeatedly (8×) in one capture (idempotence / no hidden state) | [x] |
| C21 | all 5 | ambient `A_unbuf`: `setvbuf(stdout, NULL, _IONBF, 0)` + randomised call sequence | [x] |
| C22 | all 5 | ambient `A_line`: `setvbuf(stdout, NULL, _IOLBF, 1024)` + randomised call sequence | [x] |
| C23 | all 5 | ambient `A_pipe` (default when fd 1 is a file): fully buffered + randomised call sequence — the baseline used by rows C01‥C20 | [x] |
| C24 | `printLine`, `printIntLine` | **interleaving of the two libraries into one shared buffer**: alternate C-call / Rust-call on the *same* `stdout` without an intervening flush, then compare the merged stream against the doubled expectation (catches any per-library buffering divergence such as `puts` vs `printf`) | [x] |

All 24 rows checked → Phase B complete.

## Row → test mapping

Every row is implemented by the identically-numbered test in
`tests/phase_b_configs.rs`:

| rows | tests |
|------|-------|
| C01‥C08 | `c01_print_int_line_zero` … `c08_print_int_line_full_range_sweep` |
| C09‥C15 | `c09_print_line_empty` … `c15_print_line_large_payloads` |
| C16‥C18, C20 | `c16_bad_preserves_cwe482_defect`, `c17_good`, `c18_driver_full_pipeline`, `c20_driver_repeated` |
| C19 | `c19_random_mixed_sequences` (300 randomised scripts of 1‥40 mixed low-level calls) |
| C21‥C23 | `c21_unbuffered_stdout`, `c22_line_buffered_stdout`, `c23_fully_buffered_stdout` |
| C24 | `c24_interleaved_shared_buffer` (4 buffering modes × 40 scripts) |

Each row asserts **two** things: (a) the C `.so` and the Rust `.so` produce
byte-identical output, and (b) that output matches an independent reference model
of the C semantics (`phase_b_configs.rs::model`), so a row cannot pass vacuously
by both sides emitting nothing. `tests/phase_a_selfcheck.rs` contains the
negative controls that prove the harness observes real output and does detect a
deliberate divergence.

## Run everything

```
bash translation/verify_all.sh
```

builds the C `.so`, enumerates the feature combinations out of `Cargo.toml`, and
runs `cargo check` / `cargo build` / `nm -D` symbol-diff / `cargo test` for each
combination in both the `debug` and `release` profiles.
