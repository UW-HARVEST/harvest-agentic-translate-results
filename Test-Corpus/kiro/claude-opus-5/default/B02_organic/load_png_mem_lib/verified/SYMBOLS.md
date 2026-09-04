# SYMBOLS.md — exported-symbol parity

Generated mechanically from:

```
nm -D -S --defined-only c_src/build/libharvest-work-lglu1X.so
nm -D -S --defined-only translation/target/release/libload_png_mem_lib.so
```

`c_src/CMakeLists.txt` derives the library name from the *parent directory* name
(`cmake_path(GET parent FILENAME project_name)`), so the C `.so` is
`libharvest-work-lglu1X.so` in this checkout. The Rust `.so` is
`libload_png_mem_lib.so` (`[lib] name = "load_png_mem_lib"`).

## Dynamic symbol table (defined)

| # | symbol | C type/size | Rust type/size | present in Rust | notes |
|---|--------|-------------|----------------|-----------------|-------|
| 1 | `cp_error_reason`      | `B` 0x08  | `B` 0x08  | yes | `const char *` in `.bss`; `pub static mut cp_error_reason: *const c_char` |
| 2 | `cp_dist_base`         | `D` 0x80  | `D` 0x80  | yes | `uint32_t[30+2]` = 32 * 4 = 128 bytes |
| 3 | `cp_dist_extra_bits`   | `D` 0x20  | `D` 0x20  | yes | `uint8_t[30+2]` = 32 bytes |
| 4 | `cp_fixed_table`       | `D` 0x140 | `D` 0x140 | yes | `uint8_t[288+32]` = 320 bytes |
| 5 | `cp_len_base`          | `D` 0x7c  | `D` 0x7c  | yes | `uint32_t[29+2]` = 31 * 4 = 124 bytes |
| 6 | `cp_len_extra_bits`    | `D` 0x1f  | `D` 0x1f  | yes | `uint8_t[29+2]` = 31 bytes |
| 7 | `cp_permutation_order` | `D` 0x13  | `D` 0x13  | yes | `uint8_t[19]` = 19 bytes |
| 8 | `cp_inflate`           | `T`       | `T`       | yes | `int cp_inflate(void*, int, void*, int)` |
| 9 | `load_png_mem`         | `T`       | `T`       | yes | `cp_image_t load_png_mem(const uint8_t*, int)` — 16-byte `#[repr(C)]` struct returned in `rax:rdx` |

**Missing from Rust `.so`: none.** All 9 C dynamic symbols are exported by the
Rust `.so` under the exact same names, all `D`/`B` data symbols with byte-identical
sizes. Contents of the 6 tables are compared byte-for-byte through `dlsym` in
`tests/phase_d_parity.rs`.

## Static (internal, non-exported) C functions

These are `static` in `src/lib.c` and therefore absent from `nm -D` for **both**
libraries. They are translated in `translation/src/lib.rs` as private Rust `fn`s
and are exercised indirectly through `cp_inflate` / `load_png_mem`:

`cp_make_pixel_a`, `cp_make_pixel`, `cp_would_overflow`, `cp_ptr`,
`cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`,
`cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`, `cp_paeth`,
`cp_make32`, `cp_chunk`, `cp_find`, `cp_unfilter`, `cp_convert`,
`cp_get_alpha_for_indexed_image`, `cp_depalette`, `cp_get_chunk_byte_length`,
`cp_out_size`.

## Completeness of the translation (verified mechanically)

`c_src/` contains exactly one translation unit (`src/lib.c`, 758 lines) and one
header (`include/lib.h`), so there is no whole module that could have been
skipped. Checked with:

```
# 26 function definitions in the C ...
grep -oP '^(static )?[a-zA-Z_][\w \*]*?\b(cp_[a-z0-9_]+|load_png_mem)(?=\()' c_src/src/lib.c \
  | grep -oP '(cp_[a-z0-9_]+|load_png_mem)$' | sort -u
# ... every one of which has a `fn <name>` in the Rust
```

Result: **0 missing**. All 26 C functions, all 7 globals (`cp_error_reason` +
the 6 tables) and all 4 struct types (`cp_pixel_t`, `cp_image_t`, `cp_state_t`,
`cp_raw_png_t`) are present. `cp_state_t` is `#[repr(C)]` with the same field
order, so its size (2464 bytes) and the offsets `cp_dynamic` uses
(`lit` = 0x448, `len` = 0x948, `nlen` = 0x99c, confirmed against `objdump -d`)
match the C exactly — which matters because `cp_decode` reads `tree[-1]`.

The 26 `cp_error_reason` assignment sites (25 distinct strings — "invalid image
size found" is used twice) were extracted from both sources and compared as
sets: **identical, byte for byte**. Verified in `tests/phase_c_errors.rs`, which
reads the pointer through `dlsym` and compares the C strings.

Nothing is stubbed: there is no `unimplemented!`, `todo!` or `panic!` in
`src/lib.rs`.

## Undefined symbols

C `.so` imports only: `__assert_fail`, `calloc`, `free`, `malloc`, `memcmp`,
`memcpy`, `memset` (+ weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

Rust `.so` imports the same libc allocator/`mem*` set (`bcmp` in place of
`memcmp`, `abort` in place of `__assert_fail`) plus the Rust standard
library's own libc/`_Unwind_*` dependencies. **0 missing / undefined non-libc
symbols.**

## Verification status

Automated by `tests/phase_d_parity.rs`:

* `symbol_diff_is_empty` — parses `nm -D -S --defined-only` for both `.so`s and
  asserts the C→Rust symbol difference is **empty**, with matching `nm` kind
  (`T`/`D`/`B`) and, for data symbols, matching byte size. Reports 0 Rust-only
  dynamic symbols as well.
* `no_unexpected_undefined_symbols` — asserts the Rust `.so`'s 49 imports are
  all libc / `_Unwind_*` / weak loader symbols; **0 missing non-libc symbols**.
* `exported_table_contents_match` — reads all six tables through `dlsym` and
  compares them byte for byte; also asserts `cp_error_reason` is NULL in a
  freshly loaded library on both sides.
* `exported_tables_are_writable` — confirms the tables are in writable `.data`
  in both libraries (the C exports them as `D`, not `R`), which the Phase B
  table-mutation rows depend on.

Last run: **4/4 passed**, symbol diff empty, under both the release and the
debug Rust `.so`.
