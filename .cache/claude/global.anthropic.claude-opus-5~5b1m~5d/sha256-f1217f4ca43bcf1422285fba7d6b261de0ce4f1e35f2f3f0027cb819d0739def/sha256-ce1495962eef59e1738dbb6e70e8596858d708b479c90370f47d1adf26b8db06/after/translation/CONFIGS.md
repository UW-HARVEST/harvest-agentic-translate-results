# Phase A.3 — Configuration surface table (valid inputs)

## Axes the C code actually branches on

Derived mechanically from `c_src/src/driver.c`. The library contains exactly
**two** `if`s and no `switch`, no `#ifdef` (other than the `DRIVER_H_` include
guard), no option setters, no init/teardown, no handles or contexts, no modes or
flags, no byte-order or width choices — and `Cargo.toml` declares no
`[features]`, so there is no compile-time axis either.

| axis | where in the C | values the C distinguishes |
|------|----------------|----------------------------|
| A. `line` nullness | `printLine`: `if (line != NULL)` | `NULL` / non-`NULL` |
| B. `useGood` truthiness | `driver`: `if (useGood)` | `0` / any non-`0` |
| C. entry point | the 4 exported symbols | `printLine` (lowest level), `bad` and `good` (mid level), `driver` (the top-level convenience wrapper, and the only header-declared one) |
| D. `line` byte shape | the `printf("%s\n", line)` data path — gcc lowers it to `puts(line)` | empty / 1 byte / many bytes / embedded NUL / high bytes `0x80..0xFF` / `%` format chars / newlines / very long / every one of the 255 non-NUL byte values / offset & unaligned pointers |
| E. call multiplicity and order | no globals, no state — so this must be *proven* rather than assumed | 1 call / N calls / interleaved across entry levels |
| F. caller stack contents | not a branch, but `bad()`'s indeterminate read makes it an input | clean / dirtied with 5 fill patterns × 3 recursion depths |

Axis F deserves emphasis: it is the axis that a naive translation silently gets
wrong, and it is only reachable through the *lowest-level* entry points, not
through the convenience wrapper alone.

## The table

Pruned cross-product of the axes above — one row per combination the C treats
differently. Each row is driven through both `.so`s' exports and compared
byte-for-byte; randomized rows use a fixed seed (`0x5EED_1234_ABCD_0001`) so
failures are reproducible.

| #  | entry point(s) | configuration (options set + input shape) | test (`tests/configs.rs`) | [x] |
|----|----------------|-------------------------------------------|---------------------------|-----|
| C1 | `printLine` | non-null, **empty** string (`""`) — the "zero length" boundary | `cfg_c1_printline_empty` | [x] |
| C2 | `printLine` | non-null, **1-byte** string, swept over **all 255** non-NUL byte values | `cfg_c2_printline_all_single_bytes` | [x] |
| C3 | `printLine` | non-null, **many bytes**: 512 randomized ASCII payloads, lengths 0..=64 | `cfg_c3_printline_random_ascii` | [x] |
| C4 | `printLine` | non-null, 512 randomized **full-byte-range** payloads (`0x01..=0xFF`, non-UTF-8 included), lengths 1..=128 | `cfg_c4_printline_random_bytes` | [x] |
| C5 | `printLine` | non-null, payload with an **embedded NUL** at every position 0..=32 (truncation shape) | `cfg_c5_printline_embedded_nul_sweep` | [x] |
| C6 | `printLine` | non-null, payload of **format specifiers** (`%s %d %n %% %1$s %.*s` …) passed as the *argument* | `cfg_c6_printline_format_chars` | [x] |
| C7 | `printLine` | non-null, payload containing **newlines / CR / tab / VT / FF** (output framing shape) | `cfg_c7_printline_whitespace` | [x] |
| C8 | `printLine` | non-null, **long** payloads at and around the stdio buffer sizes: 1 KiB, 4095/4096/4097, 65535/65536/65537, 1 MiB | `cfg_c8_printline_long` | [x] |
| C9 | `printLine` | non-null pointer into the **middle** of a buffer, every offset 0..36 (offset / unaligned pointer shape) | `cfg_c9_printline_offset_pointer` | [x] |
| C10 | `printLine` | `NULL` — the false branch of the only `if` in `printLine`, checked for bytes *and* clean termination | `cfg_c10_printline_null` | [x] |
| C11 | `good` | no inputs — the fixed `"string"` literal path, once and repeated | `cfg_c11_good` | [x] |
| C12 | `bad` | no inputs — the uninitialised-`data` path (defect preserved), 8 consecutive invocations | `cfg_c12_bad` | [x] |
| C13 | `driver` | `useGood = 1` ⇒ the composed `driver`→`good`→`printLine`→`puts` pipeline | `cfg_c13_driver_one` | [x] |
| C14 | `driver` | `useGood = 0` ⇒ the composed `driver`→`bad`→`printLine`→`puts` pipeline | `cfg_c14_driver_zero` | [x] |
| C15 | `driver` | `useGood` over **512 randomized `i32`** (full range incl. negatives) plus every value in `-4..=4` | `cfg_c15_driver_random_i32` | [x] |
| C16 | `driver` | `useGood` at the `i32` **boundaries**: `MIN`, `MIN+1`, `-65537`, `-65536`, `-256`, `-1`, `0`, `1`, `255`, `256`, `65535`, `65536`, `MAX-1`, `MAX` | `cfg_c16_driver_boundaries` | [x] |
| C17 | `driver` + `printLine` + `good` | **interleaved** 256-step randomized script across entry levels, compared as one whole stdout stream (framing + no hidden state) | `cfg_c17_interleaved_sequence` | [x] |
| C18 | `good`, `driver`, `printLine`, `bad` | mid-level entry points called **100×** each — idempotence / no drift | `cfg_c18_repeat_no_drift` | [x] |
| C19 | all four | caller stack **pre-dirtied** with 5 fill patterns × 3 recursion depths (the axis `bad()`'s indeterminate read is sensitive to); the well-defined paths must be *unaffected*, the defective one must diverge *identically* | `cfg_c19_dirty_stack_matrix` | [x] |
| C19b | `good`+`bad`, `driver`+`driver`, `printLine`+`bad` | **frame aliasing**: `good`'s spill of the string literal lands in exactly the slot a following `bad()` reads, so the C prints `string` twice — the sharpest test of frame-layout parity | `cfg_c19b_good_then_bad_frame_aliasing` | [x] |
| C20 | `printLine` | **fuzz**: 1024 randomized payloads mixing all of the shapes above | `cfg_c20_printline_fuzz_mixed` | [x] |

## Results

All 21 rows pass, for every feature combination (`default`,
`--no-default-features`, `--all-features`) and both profiles (`dev`, `release`),
via `./verify.sh`.

Two real divergences were found by this table and fixed in the Rust (never in the
C); both were exposed by rows C12/C14/C18/C19/C19b, i.e. the ones that reach the
lowest-level entry points with non-trivial stack state:

1. `bad()` did not reproduce the indeterminate stack read at all (it substituted
   a deterministic empty string), so `good(); bad();` printed `string\n\n`
   instead of the C's `string\nstring\n`, and a dirtied stack made the C fault
   where the Rust did not.
2. The Rust `.so` was linked `-z now` while the C's is lazily bound, which
   changed the dynamic linker's stack residue that `driver(0)` observes.

See `SYMBOLS.md` for both fixes, and `ERRORS.md` for the mutation testing that
confirms these rows genuinely fail when the translation is wrong.

`tests/probe_ub.rs` (run with `-- --ignored --nocapture`) prints the raw
side-by-side behaviour of every stack-sensitive case, which is how one can see
that these rows are not degenerate: e.g. for `printLine("AAAA…"); bad();` both
libraries dump the same ~60 bytes of stale machine code, and the dirty-stack
matrix produces real `SIGSEGV`s on both sides.
