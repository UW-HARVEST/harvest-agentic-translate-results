# CONFIGS.md — Phase B configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Axes derived mechanically from the C
source, not guessed.

## Axes the C code actually distinguishes

1. **Entry points (all five exported symbols, lowest-level first).** The public
   header only declares `driver`, but `printLine`, `printIntLine`, `bad` and
   `good` all have external linkage and are exported by the `.so`, so all five
   are driven directly through `dlsym` — not just the `driver` one-shot wrapper.
   Call hierarchy from the source:
   - level 0 (leaf): `printLine`, `printIntLine` → libc `printf`
   - level 1: `good`, `bad` → `printIntLine`
   - level 2 (composed pipeline): `driver` → `printLine`, `good`, `bad`
2. **Runtime options / modes / flags: none.** There are no setters, no globals,
   no `static` state, no environment reads, and no `#ifdef`s in the C source, so
   there is no option cross-product to enumerate. The only branch in the whole
   library is `printLine`'s `line != NULL` (its false arm is `ERRORS.md` row E1).
3. **Input shapes** — the only remaining axis, per parameter type:
   - `const char *line`: NULL vs non-NULL; length 0 / 1 / many; content class
     (printable ASCII, whitespace/control, embedded newlines, `printf` format
     specifiers, high non-UTF-8 bytes 0x80–0xFF); length relative to the libc
     stream buffer (4 KiB) — under, exactly at, over.
   - `int intNumber`: sign (negative / zero / positive), decimal width 1–10
     digits, and the `INT_MIN`/`INT_MAX` extremes.
   - `bad`/`good`/`driver`: no parameters, so their shape axis is *invocation
     pattern* — single call, repeated calls (checking there is no hidden state),
     and interleaving with the leaf functions in one `stdout` buffer.
4. **Byte order / element width / count / format:** the API has no
   multi-byte-element, array, or endianness surface (no `struct`, no buffers, no
   counts), so those axes collapse to the two scalar shapes above.

Comparison method for every row: redirect fd 1 to a file, call the symbol from
the C `.so`, `fflush`, capture bytes; repeat identically with the Rust `.so`;
assert the two byte vectors are equal. Randomized rows use a `SplitMix64` PRNG
with a **fixed seed (0x5EED_1234_ABCD_EF01)** so failures reproduce.

## Configuration table

| # | entry point(s) | configuration (options set + input shape) | cases | test | [x] |
|---|----------------|-------------------------------------------|-------|------|-----|
| 1 | `printLine` | non-NULL, empty string `""` | 1 | `cfg_01_print_line_empty` | [x] |
| 2 | `printLine` | non-NULL, single character — every byte value `0x01`–`0xFF` | 255 | `cfg_02_print_line_single_byte` | [x] |
| 3 | `printLine` | random printable ASCII (0x20–0x7E), random length 1–256 | 512 | `cfg_03_print_line_random_ascii` | [x] |
| 4 | `printLine` | content containing `printf` format specifiers (`%s`, `%d`, `%n`, `%%`, `%99999999d`, `%p`), fixed + randomly assembled | 6 + 256 | `cfg_04_print_line_format_specifiers` | [x] |
| 5 | `printLine` | content with embedded control/whitespace bytes: `\n`, `\r`, `\t`, `\x0b`, `\x0c`, `\x7f`, randomly placed | 256 | `cfg_05_print_line_embedded_control` | [x] |
| 6 | `printLine` | random arbitrary bytes `0x01`–`0xFF` (non-UTF-8), random length 1–512 | 512 | `cfg_06_print_line_random_bytes` | [x] |
| 7 | `printLine` | lengths straddling the libc 4 KiB stream buffer: 1, 2, 4095, 4096, 4097, 8191, 8192, 8193, 65535, 65536, 1 MiB | 11 | `cfg_07_print_line_buffer_boundaries` | [x] |
| 8 | `printIntLine` | `0` | 1 | `cfg_08_print_int_zero` | [x] |
| 9 | `printIntLine` | `1`, `-1` (sign axis at the smallest magnitude) | 2 | `cfg_09_print_int_plus_minus_one` | [x] |
| 10 | `printIntLine` | `INT_MAX` (`2147483647`), `INT_MIN` (`-2147483648`) | 2 | `cfg_10_print_int_extremes` | [x] |
| 11 | `printIntLine` | every decimal-width boundary: `±10^k` and `±(10^k - 1)` for k = 1..9 | 36 | `cfg_11_print_int_width_boundaries` | [x] |
| 12 | `printIntLine` | uniform random `i32` over the full 32-bit range | 2048 | `cfg_12_int_random` | [x] |
| 13 | `printIntLine` | random small-magnitude values (-1000..1000), where sign/width changes densely | 1024 | `cfg_13_print_int_random_small` | [x] |
| 14 | `good` | single call (level-1 entry point, exercised directly) | 1 | `cfg_14_good_single` | [x] |
| 15 | `bad` | single call (level-1; preserves the original's discarded-value bug) | 1 | `cfg_15_bad_single` | [x] |
| 16 | `driver` | single call — the full composed pipeline `printLine` + `good` + `bad` | 1 | `cfg_16_driver_single` | [x] |
| 17 | `good`, `bad`, `driver` | repeated invocation, 32× each in one capture (proves no hidden/carried state, and that repeat output is identical) | 3 | `cfg_17_repeated_invocations` | [x] |
| 18 | all five, mixed | randomized sequences of 1–24 calls drawn from all five entry points with randomized arguments, captured as one `stdout` stream (exercises the composed pipeline and buffering interaction, not one call at a time) | 256 sequences | `cfg_18_random_mixed_sequences` | [x] |

Rows 1–7 and 18 also cover `printLine`'s true branch at `driver.c:31`; the false
branch is `ERRORS.md` E1.

## Harness validation (negative controls)

Passing tests only mean something if the harness can fail. Four deliberately
broken Rust libraries were built and pointed at via `RUST_DRIVER_SO=…`, without
touching the real crate. Each was caught, by the phase that should catch it:

| mutant | injected defect | caught by |
|---|---|---|
| M1 | `bad()` assigns `intOne + intTwo` (i.e. "fixes" the original's discarded-value bug) | Phase B rows 15, 16, 16b, 17, 18 — `DIVERGENCE [bad()] at byte 2` |
| M2 | `printLine` passes `line` as the `printf` **format string** instead of an argument | Phase B rows 1–7, 16–18 and 6 Phase C tests |
| M3 | `printLine`'s NULL guard weakened to print an empty line for NULL | Phase C `err_e1_print_line_null` (`Rust printLine(NULL) unexpectedly wrote "\n"`), `err_g1_null_interleaved`, and Phase B row 18 |
| M4 | `#[no_mangle]` removed from `good` so it is not exported | Phase D `phase_d_symbol_parity_is_exact` and `phase_d_every_c_symbol_is_dlsym_resolvable_in_rust` |

The unmodified crate passes all 32 tests; each mutant fails. So the captures are
comparing real bytes, not two empty buffers.

## How to reproduce

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && ./scripts/verify_all.sh      # every feature combo × dev/release
cd translation && cargo build && cargo test -- --test-threads=1
```
