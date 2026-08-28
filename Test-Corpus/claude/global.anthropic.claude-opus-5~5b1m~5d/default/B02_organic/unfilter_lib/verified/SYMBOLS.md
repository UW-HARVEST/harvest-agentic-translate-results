# SYMBOLS.md — Phase A: exported-symbol surface

Reference C shared library (built exactly as the task describes, i.e. **no**
`CMAKE_BUILD_TYPE`, therefore **`NDEBUG` is *not* defined and `assert()` is
live**):

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-81cf2F.so      (name = parent directory name)
```

Rust shared library:

```
cd translation && cargo build --release
# -> translation/target/release/libunfilter_lib.so
```

## Dynamic symbol table (`nm -D --defined-only`)

| # | symbol | type | C size | Rust size | C | Rust | notes |
|---|--------|------|--------|-----------|---|------|-------|
| 1 | `unfilter`            | `T` (func) | 0x470 | 0x1744 | ✅ | ✅ | the only symbol declared by `include/lib.h` |
| 2 | `cp_inflate`          | `T` (func) | 0x29b | 0xd58  | ✅ | ✅ | not in the public header, but exported |
| 3 | `cp_error_reason`     | `B` (data, bss) | 8 | 8 | ✅ | ✅ | `const char *`, mutable global, written by the error paths |
| 4 | `cp_fixed_table`      | `D` (data) | 0x140 = 320 | 0x140 | ✅ | ✅ | `uint8_t[288+32]` |
| 5 | `cp_permutation_order`| `D` (data) | 0x13 = 19   | 0x13  | ✅ | ✅ | `uint8_t[19]` |
| 6 | `cp_len_extra_bits`   | `D` (data) | 0x1f = 31   | 0x1f  | ✅ | ✅ | `uint8_t[29+2]` |
| 7 | `cp_len_base`         | `D` (data) | 0x7c = 124  | 0x7c  | ✅ | ✅ | `uint32_t[29+2]` |
| 8 | `cp_dist_extra_bits`  | `D` (data) | 0x20 = 32   | 0x20  | ✅ | ✅ | `uint8_t[30+2]` |
| 9 | `cp_dist_base`        | `D` (data) | 0x80 = 128  | 0x80  | ✅ | ✅ | `uint32_t[30+2]` |

`nm -D` symbol-name diff between the two libraries: **empty** (verified by
`tests/symbols_diff.rs`, which shells out to `nm` at test time and also
`dlsym()`s every C symbol out of the Rust `.so`).

Both `.so`s also export the usual per-object housekeeping symbols
(`_ITM_*`, `__gmon_start__`, `__cxa_finalize`, …) as *undefined weak*
references, not definitions, so they are not part of the surface.

## `static` (internal, non-exported) C functions

These are file-local in C (`static`), hence absent from `nm -D` in **both**
libraries. All of them are translated in `translation/src/lib.rs` and are
reachable only through `cp_inflate` / `unfilter`:

| C symbol | Rust counterpart | reachable from |
|----------|------------------|----------------|
| `cp_make_pixel_a`   | `cp_make_pixel_a`   | dead code in C too |
| `cp_make_pixel`     | `cp_make_pixel`     | dead code in C too |
| `cp_would_overflow` | `cp_would_overflow` | only used inside `assert()` |
| `cp_ptr`            | `cp_ptr`            | `cp_stored` |
| `cp_peak_bits`      | `cp_peak_bits`      | `cp_read_bits`, `cp_decode` |
| `cp_consume_bits`   | `cp_consume_bits`   | `cp_read_bits`, `cp_decode` |
| `cp_read_bits`      | `cp_read_bits`      | everywhere |
| `cp_rev16`          | `cp_rev16`          | `cp_build`, `cp_decode` |
| `cp_build`          | `cp_build`          | `cp_fixed`, `cp_dynamic` |
| `cp_stored`         | `cp_stored`         | `cp_inflate` (btype 0) |
| `cp_fixed`          | `cp_fixed`          | `cp_inflate` (btype 1) |
| `cp_decode`         | `cp_decode`         | `cp_dynamic`, `cp_block` |
| `cp_dynamic`        | `cp_dynamic`        | `cp_inflate` (btype 2) |
| `cp_block`          | `cp_block`          | `cp_inflate` (btype 1, 2) |
| `cp_paeth`          | `cp_paeth`          | `unfilter` |
| `cp_make32`         | `cp_make32`         | `cp_chunk`, `cp_find` (both dead in C) |
| `cp_chunk`          | `cp_chunk`          | dead code in C too |
| `cp_find`           | `cp_find`           | dead code in C too |

Types `cp_pixel_t`, `cp_image_t`, `cp_state_t`, `cp_raw_png_t` are all
translated as `#[repr(C)]` structs. `cp_state_t`'s layout matters for
behavioural parity (see `CONFIGS.md`, row group "state-layout"): `cp_decode`
reads `tree[lo - 1]`, which for `lo == 0` reads the `u32` *preceding* the
`lit` / `dst` / `len` sub-arrays inside the state struct. The Rust translation
derives those sub-array pointers from the base of the same allocation, so the
out-of-range read hits the same bytes. Verified offsets (x86-64 SysV):

| field | offset | field | offset |
|-------|--------|-------|--------|
| `bits` | 0 | `out` | 48 |
| `count` | 8 | `out_end` | 56 |
| `words` | 16 | `begin` | 64 |
| `word_count` | 24 | `lookup` | 72 |
| `word_index` | 28 | `lit` | 1096 |
| `bits_left` | 32 | `dst` | 2248 |
| `final_word_available` | 36 | `len` | 2376 |
| `final_word` | 40 | `nlit`/`ndst`/`nlen` | 2452/2456/2460 |

Total size 2464, alignment 8. `sym06_cp_state_t_layout_matches_c` derives the C
offsets at test time from a probe compiled out of the struct definition copied
verbatim from `c_src/src/lib.c` (lines 71-90) and compares all 18 of them plus
`sizeof` against a `#[repr(C)]` mirror of the translation's struct.
`cfg58_decode_reads_tree_minus_one` then checks the same thing
*behaviourally*, by driving the `tree[-1]` read itself.

## Missing symbols

None. No C source file was left untranslated: `c_src` contains exactly one
translation unit (`src/lib.c`, 478 lines) plus `include/lib.h`, and every
function/global in it has a counterpart in `translation/src/lib.rs`.

## Feature combinations

`translation/Cargo.toml` declares exactly one optional feature, `c-asserts`
(in `default`), which mirrors the C build's `NDEBUG` switch:

| combination | reproduces | C library the harness compares against |
|-------------|-----------|-----------------------------------------|
| default (`c-asserts`) | the reference `.so` from the task's cmake command (no `CMAKE_BUILD_TYPE` ⇒ `assert()` live) | `c_src/build/lib*.so` |
| `--no-default-features` | the same C source built with `-DNDEBUG` | `gcc -O0 -fPIC -DNDEBUG` build, made by the harness |
| `--no-default-features --features c-asserts` | identical to the default | `c_src/build/lib*.so` |

`run_all.sh` enumerates them mechanically from `Cargo.toml` (power set of the
non-`default` features, plus the default set) and runs the whole test suite for
each, in both the `dev` and `release` cargo profiles (the `release` profile is
the one that carries `panic = "abort"`, `overflow-checks = false`,
`debug-assertions = false`). The `.so` under test is always rebuilt with the
feature set of the running test binary, so a stale artifact from the other
combination cannot be tested by accident.

The feature does **not** change the exported symbol set: `c-asserts` only adds
internal, `static`-equivalent code (`cp_assert_fail`), verified by
`sym01_every_c_symbol_is_exported_by_rust` under both combinations.
