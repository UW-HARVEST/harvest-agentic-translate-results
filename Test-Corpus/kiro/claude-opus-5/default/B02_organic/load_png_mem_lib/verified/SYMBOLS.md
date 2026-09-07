# SYMBOLS.md — exported-symbol parity

Source of truth: `nm -D --defined-only` on both shared objects.

* C:    `c_src/build/libharvest-work-BAYhIR.so`
* Rust: `translation/target/release/libload_png_mem_lib.so`

## C `.so` public symbols (9) vs Rust `.so`

| # | symbol | C type | Rust `.so` | notes |
|---|--------|--------|------------|-------|
| 1 | `cp_inflate`          | `T` (text)  | present `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` |
| 2 | `load_png_mem`        | `T` (text)  | present `T` | returns `cp_image_t` by value (16-byte struct, SysV: RAX/RDX pair — verified identical) |
| 3 | `cp_fixed_table`      | `D` (data)  | present `D` | `[u8; 320]` |
| 4 | `cp_permutation_order`| `D` (data)  | present `D` | `[u8; 19]` |
| 5 | `cp_len_extra_bits`   | `D` (data)  | present `D` | `[u8; 31]` |
| 6 | `cp_len_base`         | `D` (data)  | present `D` | `[u32; 31]` |
| 7 | `cp_dist_extra_bits`  | `D` (data)  | present `D` | `[u8; 32]` |
| 8 | `cp_dist_base`        | `D` (data)  | present `D` | `[u32; 32]` |
| 9 | `cp_error_reason`     | `B` (bss)   | present `B` | `*const c_char`; used as the error-identity oracle in Phase C |

**Missing from Rust: none.** No module of `c_src/src/lib.c` was skipped — the
single C translation unit is fully translated (all `static` helpers plus the two
`extern` functions and seven `extern` tables).

## Static (non-exported) C functions — must NOT appear in `nm -D`

`cp_make_pixel_a`, `cp_make_pixel`, `cp_would_overflow`, `cp_ptr`,
`cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`,
`cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`, `cp_paeth`,
`cp_make32`, `cp_chunk`, `cp_find`, `cp_unfilter`, `cp_convert`,
`cp_get_alpha_for_indexed_image`, `cp_depalette`,
`cp_get_chunk_byte_length`, `cp_out_size`.

All are private (non-`no_mangle`) in the Rust translation. Verified: neither
`.so` exports any of them.

## Undefined (imported) symbols

Rust `.so` imports only libc/`ld` symbols (`malloc`, `calloc`, `free`,
`memcpy`, `memset`, `memcmp`, `abort`, plus unwind/`__cxa` runtime stubs).
0 missing/undefined non-libc symbols.

## Data-content parity

Beyond name parity, the byte contents of all 6 exported tables are compared
element-by-element between the two `.so`s in `tests/symbols.rs`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section** and no
`[[bin]]`/`src/main.rs`. There is exactly one build configuration
(default == no-default-features == all-features) and no driver binary, so
"repeat for every feature combination" collapses to the single default
configuration. This is asserted mechanically by `check_features.sh`.

---

# Verification result

Reproduce with `./check_all.sh` (builds the C `.so`, enumerates feature
combinations, checks the symbol diff, runs every suite).

```
symbol diff: empty (9 C symbols all present)
  symbols            ok  5 passed
  phase_b_inflate    ok  23 passed
  phase_b_png        ok  22 passed
  phase_c_errors     ok  33 passed
  isolated           ok  3694 cases matched
```

* `nm -D` diff C → Rust: **empty**. No symbol needed a new wrapper and no C
  module was missing, so nothing had to be translated in this step.
* `ldd -r` on both `.so`s: no unresolved symbols.
* The byte contents of all 6 exported tables are compared element-by-element
  between the two `.so`s (`tests/symbols.rs::exported_table_contents_match`).
* None of the 24 `static` C helpers is exported by either `.so`.
