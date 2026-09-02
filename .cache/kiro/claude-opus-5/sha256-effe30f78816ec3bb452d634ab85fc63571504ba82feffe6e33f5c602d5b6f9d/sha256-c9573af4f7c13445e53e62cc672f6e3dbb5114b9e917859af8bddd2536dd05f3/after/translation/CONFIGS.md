# CONFIGS.md — Configuration surface of `c_src` (valid inputs)

Mirror of `ERRORS.md`, for inputs the C **accepts**. Derived mechanically from
the source, not from assumptions about what "matters".

## Axis 1 — runtime options / modes / flags: NONE

```sh
grep -nE '^\s*(static|extern|const|enum|struct|typedef|union)|#if|#ifdef|#define' \
     c_src/src/driver.c c_src/include/driver.h
# -> only the DRIVER_H_ include guard
```

There are **no** globals, no `static` state, no setters, no mode/flag
parameters, no environment lookups, no `enum`s, and no `#ifdef`-selected code
paths. The library is fully stateless and its behaviour is a pure function of
each call's arguments. Consequently the configuration cross-product collapses
onto the **input shapes** (axis 3) and the **entry point** (axis 2). There are
also no Cargo features (`grep -c '^\[features\]' translation/Cargo.toml` → 0),
so there is exactly one build configuration; see the Feature-combination note
at the bottom.

## Axis 2 — full set of public entry points (including the lowest-level one)

```sh
grep -nE '^[a-zA-Z_].*\(' c_src/src/driver.c
# 30: void printLine (const char * line)
# 38: void driver(int data)
```

| entry point | in public header? | exported by `.so`? | level |
|---|---|---|---|
| `driver(int data)` | yes | yes | high level / one-shot wrapper — builds the buffers and then calls `printLine` |
| `printLine(const char *line)` | **no** (absent from `driver.h`) | **yes** (not `static`) | **lowest level** — the primitive that performs the actual output |

`printLine` is the low-level entry point that the convenience wrapper composes
with, so it is driven **directly** below (rows 8–15), not only through `driver`.

## Axis 3 — input shapes the C special-cases

Branch points that distinguish shapes (`driver.c:32` and `driver.c:44`), plus
the constants `100` / `99` from `driver.c:40-43`:

* `driver`: the accepted window is the signed range `[0, 99]`, split by the
  distinguishable sub-shapes **empty (0)**, **one (1)**, **many (2..98)**, and
  **exactly-full (99)** — at `data == 99` the copy consumes all 99 `'A'` bytes
  of `source` and `dest[99]` is the last in-bounds element.
* `strncpy`'s NUL-copy behaviour depends on whether `n` exceeds the source
  length; `source` is 99 non-NUL bytes + NUL at `[99]`, so for every accepted
  `data <= 99` **no** terminator is copied and the explicit `dest[data] = '\0'`
  is what terminates the result. `data == 99` is therefore the boundary between
  "terminator supplied by `strncpy`" and "not", and is called out separately.
* `printLine`: the only branch is NUL-vs-non-NUL pointer (`ERRORS.md` row 1);
  for a non-NULL pointer the shape axes that reach `stdio` are **string
  length** (empty / one / many / longer than the `BUFSIZ` stdio buffer, which
  forces a different number of `write(2)` calls) and **byte content**
  (including `%` — inert because the format string is fixed —, embedded `\n`,
  `\t`, `\r`, high-bit / non-UTF-8 bytes, and byte value `0xFF`).

No other shape axes exist: the only types in the API are `char` and `int`, so
there are no element widths, byte-order, count, or format axes to enumerate.

## Configuration table

One row per combination the C actually treats differently. Every row is
exercised with **many randomised inputs** (fixed seed `0x5EED_D00D`), not one
hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | expected observable | test | [x] |
|---|----------------|-------------------------------------------|---------------------|------|-----|
| 1 | `driver` | accepted range `[0,99]`, **randomised sweep** over the whole window | `"A" * data + "\n"` | `cfg_01_driver_accepted_range_random` | [x] |
| 2 | `driver` | accepted range `[0,99]`, **exhaustive** every value 0..=99 | `"A" * data + "\n"` | `cfg_02_driver_accepted_range_exhaustive` | [x] |
| 3 | `driver` | shape **empty**: `data == 0` (zero length; `strncpy` copies nothing) | `"\n"` (1 byte) | `cfg_03_driver_data_zero` | [x] |
| 4 | `driver` | shape **one**: `data == 1` | `"A\n"` (2 bytes) | `cfg_04_driver_data_one` | [x] |
| 5 | `driver` | shape **many**: `data` randomised in `[2,98]` | `"A" * data + "\n"` | `cfg_05_driver_data_many_random` | [x] |
| 6 | `driver` | shape **exactly-full**: `data == 99` — copy consumes all of `source`, `dest[99]` is the last in-bounds byte, no terminator came from `strncpy` | `"A" * 99 + "\n"` (100 bytes) | `cfg_06_driver_data_99_full` | [x] |
| 7 | `driver` | **repeated / stateful** invocation: the same randomised `data` sequence called many times in a row on one loaded handle, verifying the library really is stateless and that no residue leaks between calls | concatenation of the per-call outputs, identical for C and Rust | `cfg_07_driver_repeated_calls_random` | [x] |
| 8 | `printLine` (low level, direct) | shape **empty**: `line = ""` | `"\n"` (1 byte) | `cfg_08_printline_empty` | [x] |
| 9 | `printLine` (low level, direct) | shape **one**: single-byte string, randomised over all 255 non-NUL byte values | `<byte> + "\n"` | `cfg_09_printline_one_byte_all_values` | [x] |
| 10 | `printLine` (low level, direct) | shape **many**: randomised length `[2,4096]`, randomised bytes drawn from the full non-NUL range `1..=255` | `line + "\n"` | `cfg_10_printline_many_random_bytes` | [x] |
| 11 | `printLine` (low level, direct) | shape **`driver`-like**: `"A" * n` for randomised `n` in `[0,99]` — the exact buffer contents `driver` synthesises, driven through the primitive directly | `"A" * n + "\n"` | `cfg_11_printline_driver_shaped_buffer` | [x] |
| 12 | `printLine` (low level, direct) | content **format-specifier bytes**: strings containing `%s`, `%n`, `%d`, `%%`, `%` — inert because `driver.c:34` passes a *fixed* format string, but a real divergence risk if a translation ever passed `line` as the format | the string verbatim + `"\n"` | `cfg_12_printline_percent_content` | [x] |
| 13 | `printLine` (low level, direct) | content **embedded control bytes**: `\n`, `\t`, `\r`, `\x0b`, `\x7f` at randomised positions | the string verbatim + `"\n"` | `cfg_13_printline_control_bytes` | [x] |
| 14 | `printLine` (low level, direct) | shape **longer than the stdio buffer**: randomised length `[8193, 40960]` (> `BUFSIZ`), forcing multiple `write(2)` flushes | the string verbatim + `"\n"` | `cfg_14_printline_over_bufsiz` | [x] |
| 15 | `printLine` (low level, direct) | content **high-bit / invalid-UTF-8**: bytes `0x80..=0xFF` only, incl. `0xFF`, randomised — a Rust translation that round-tripped through `str`/`String` would corrupt or panic here | the bytes verbatim + `"\n"` | `cfg_15_printline_high_bit_bytes` | [x] |
| 16 | `driver` + `printLine` **composed pipeline** | interleaved randomised call sequence mixing both entry points (and NULL `printLine` calls, which emit nothing) on a single handle — checks output **ordering and stdio buffering** across the composed pipeline, which per-function tests cannot see | the exact interleaved byte stream, identical for C and Rust | `cfg_16_interleaved_pipeline_random` | [x] |

## Feature-combination note

`translation/Cargo.toml` declares **no `[features]` section**, so
`--no-default-features` and any `--features <combo>` are equivalent to the
default build; the only build configuration is the default one. This is verified
mechanically by `tests/feature_matrix.sh`, which extracts the feature list from
`cargo metadata` and runs the full suite for every combination it finds
(default, `--no-default-features`, and `--all-features`).
