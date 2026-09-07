# CONFIGS.md — Configuration-surface table (Phase A, gate for Phase B)

Derived **mechanically** from the C source, the same way `ERRORS.md` is.

## Step 1 — enumerate the axes the C actually distinguishes

### Axis: public entry points (full set, including the lowest level)

```sh
grep -nE '^[A-Za-z_].*\(' c_src/include/lib.h   # -> char *custom_strdup(const char *str);
nm -D --defined-only c_src/build/libdriver.so   # -> T custom_strdup
```

`custom_strdup` is the **only** public entry point. It is simultaneously the
lowest-level and the highest-level API — there is no convenience wrapper layered
over a lower primitive, so "test the low-level entry points, not just the
wrappers" collapses to "test `custom_strdup`". Its three internal callees
(`strlen`, `malloc`, `memcpy`) come from libc and are not part of this library.

### Axis: runtime options / modes / flags

**Empty.** `grep -cE 'enum|#define|static|extern|struct|flag|mode|option|setopt' c_src/src/lib.c c_src/include/lib.h` finds no configuration state: no global, no init function, no setopt-style call, no `enum`, no flags parameter, no `#ifdef`. The function is pure w.r.t. configuration — its behaviour is a function of the single `str` argument and the allocator's success only.

### Axis: `#if` / `#ifdef` conditional compilation

**Empty** in both C files. `Cargo.toml` declares no `[features]`, so the Rust side likewise has a single configuration. The full "feature combination" cross-product is therefore the single default build (see `SYMBOLS.md`).

### Axis: input shapes the code is sensitive to

The C body is straight-line after the two null checks, but its behaviour is still shape-dependent through `strlen` (where the copy stops) and `memcpy` (which selects different SIMD/word-at-a-time paths by size and alignment). Shapes enumerated:

* **pointer validity** — `NULL` vs valid buffer (the `if(!str)` branch);
* **length** — 0 (empty), 1, 2, machine-word and SIMD boundaries (7/8/9, 15/16/17, 31/32/33, 63/64/65, 127/128/129), page boundaries (4095/4096/4097, 8191/8192/8193), large (1 MiB), huge (16 MiB);
* **byte content** — ASCII, high bytes ≥ 0x80 (signedness of `char`!), 0xFF, the complete non-NUL byte domain `1..=255`;
* **content past the terminator** — a buffer whose bytes *after* the NUL are non-zero garbage, to prove exactly `strlen+1` bytes are copied and no more;
* **source alignment** — the string starting at each offset 0..15 of an aligned buffer;
* **read-boundary** — the NUL as the final readable byte before an unmapped guard page, to prove no over-read;
* **count / lifetime** — one call, many simultaneously live results, many sequential calls, concurrent calls from several threads;
* **allocator ABI** — the result must be releasable with `free`, since the C hands back a `malloc` buffer and that is observable to every caller.

Byte order / element type / element width are not axes: the function copies raw bytes and never interprets multi-byte values.

## Step 2 — pruned cross-product

One row per combination the C treats differently. Every row is exercised against **both** `.so`s via `libloading` with **many seeded-random inputs** (seed `0x5EED_C0FFEE_u64`, `SplitMix64`), not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `custom_strdup` | no options (none exist) × `str = NULL` | `cfg_row01_null_pointer` | [x] |
| 2 | `custom_strdup` | × empty string `""` (length 0 → `len == 1`) | `cfg_row02_empty_string` | [x] |
| 3 | `custom_strdup` | × length exactly 1, all 255 possible single non-NUL byte values | `cfg_row03_length_one_all_bytes` | [x] |
| 4 | `custom_strdup` | × every length 0..=64, randomized ASCII content, many seeds | `cfg_row04_lengths_0_to_64_random_ascii` | [x] |
| 5 | `custom_strdup` | × machine-word / SIMD `memcpy` boundary lengths {7,8,9,15,16,17,31,32,33,63,64,65,127,128,129}, randomized full-byte-domain content | `cfg_row05_word_and_simd_boundary_lengths` | [x] |
| 6 | `custom_strdup` | × page-boundary lengths {4095,4096,4097,8191,8192,8193}, randomized content | `cfg_row06_page_boundary_lengths` | [x] |
| 7 | `custom_strdup` | × large body: 1 MiB of randomized non-NUL bytes | `cfg_row07_one_mib_random` | [x] |
| 8 | `custom_strdup` | × huge body: 16 MiB of randomized non-NUL bytes | `cfg_row08_sixteen_mib_random` | [x] |
| 9 | `custom_strdup` | × content = the complete non-NUL byte domain `1..=255` in one string (high bytes, `char`-signedness sensitive) | `cfg_row09_full_byte_domain` | [x] |
| 10 | `custom_strdup` | × content = only high bytes `0x80..=0xFF`, randomized order and length | `cfg_row10_high_bytes_only` | [x] |
| 11 | `custom_strdup` | × non-zero garbage **after** the terminator (buffer ≫ string) — exactly `strlen+1` bytes must be copied, no over-copy | `cfg_row11_garbage_after_terminator` | [x] |
| 12 | `custom_strdup` | × terminator at offset 0 inside a large populated buffer (early-NUL truncation) | `cfg_row12_early_nul_in_large_buffer` | [x] |
| 13 | `custom_strdup` | × source pointer at each misalignment 0..=15 within an aligned buffer, randomized lengths | `cfg_row13_source_misalignment` | [x] |
| 14 | `custom_strdup` | × string whose NUL is the last readable byte before an unmapped guard page (proves no read past the terminator) | `cfg_row14_guard_page_no_overread` | [x] |
| 15 | `custom_strdup` | × result must be `free`-able — allocator ABI compatibility of the returned buffer (C returns `malloc` memory; Rust must too) | `cfg_row15_result_is_free_able` | [x] |
| 16 | `custom_strdup` | × result must not alias the input and must be an independent buffer (mutate result, input unchanged; mutate input, result unchanged) | `cfg_row16_result_is_independent_copy` | [x] |
| 17 | `custom_strdup` | × many simultaneously live results (256 outstanding buffers, all distinct, all intact) — no shared scratch buffer | `cfg_row17_many_live_results` | [x] |
| 18 | `custom_strdup` | × concurrent calls from 8 threads, randomized inputs (reentrancy / no shared mutable state) | `cfg_row18_concurrent_calls` | [x] |
| 19 | `custom_strdup` | × unrestricted property sweep: 20 000 seeded-random inputs, length 0..=4096, bytes drawn from `1..=255`, ~1/50 of them `NULL` — mixes all axes above | `cfg_row19_property_sweep` | [x] |

## Binary / driver executable

`c_src/CMakeLists.txt` contains **no `add_executable`** (only
`add_library(driver SHARED src/lib.c)`), and `translation/` has no `src/main.rs`
and no `[[bin]]` in `Cargo.toml`. The project builds **no binary**, so the
"compare C and Rust stdout byte-for-byte" gate item is **N/A**. Verified:

```sh
grep -c add_executable c_src/CMakeLists.txt   # -> 0
ls translation/src/                            # -> lib.rs only
```

## Harness validation (proof the rows are not vacuous)

Passing tests only mean something if they can fail. Four bugs were injected into
`src/lib.rs` in turn (then reverted; `src/lib.rs` is byte-identical to its
original), rebuilt, and the suite re-run:

| injected bug | result |
|--------------|--------|
| `len = strlen(str)` — drop the `+ 1`, so the NUL is not copied | 16 tests failed |
| `if(!str)` returns `malloc(1)` instead of `NULL` | 3 tests failed |
| return the input pointer instead of the fresh copy (aliasing) | 11 tests failed |
| `memcpy` copies `len - 1` bytes for `len > 4` (partial copy) | 12 tests failed |

Every mutant was detected by assertion failures, not by luck or by a crash.
