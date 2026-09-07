# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-M25J6k.so
nm -D --defined-only translation/target/release/libunfilter_lib.so
```

## Exported symbols of the C `.so`

| # | symbol | nm type | C declaration | in Rust `.so`? | Rust item |
|---|--------|---------|---------------|----------------|-----------|
| 1 | `cp_error_reason`      | `B` (bss)  | `const char *cp_error_reason;`            | YES | `#[no_mangle] pub static mut cp_error_reason` |
| 2 | `cp_dist_base`        | `D` (data) | `uint32_t cp_dist_base[30 + 2]`           | YES | `#[no_mangle] pub static mut cp_dist_base` |
| 3 | `cp_dist_extra_bits`  | `D` (data) | `uint8_t cp_dist_extra_bits[30 + 2]`      | YES | `#[no_mangle] pub static mut cp_dist_extra_bits` |
| 4 | `cp_fixed_table`      | `D` (data) | `uint8_t cp_fixed_table[288 + 32]`        | YES | `#[no_mangle] pub static mut cp_fixed_table` |
| 5 | `cp_len_base`         | `D` (data) | `uint32_t cp_len_base[29 + 2]`            | YES | `#[no_mangle] pub static mut cp_len_base` |
| 6 | `cp_len_extra_bits`   | `D` (data) | `uint8_t cp_len_extra_bits[29 + 2]`       | YES | `#[no_mangle] pub static mut cp_len_extra_bits` |
| 7 | `cp_permutation_order`| `D` (data) | `uint8_t cp_permutation_order[19]`        | YES | `#[no_mangle] pub static mut cp_permutation_order` |
| 8 | `cp_inflate`          | `T` (text) | `int cp_inflate(void*, int, void*, int)`  | YES | `#[no_mangle] pub extern "C" fn cp_inflate` |
| 9 | `unfilter`            | `T` (text) | `int unfilter(int, int, int, uint8_t*)`   | YES | `#[no_mangle] pub extern "C" fn unfilter` |

**Symbol diff (C exported, missing from Rust): EMPTY.**
No stubs were added; every symbol above is backed by a real translation of the C body.

## C `static` (internal, non-exported) functions — all translated

Confirmed present in the C object's `.symtab` but not `.dynsym`, therefore not
required to be exported. All are nevertheless translated in `src/lib.rs`:

`cp_make_pixel_a`, `cp_make_pixel`, `cp_would_overflow`, `cp_ptr`,
`cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`,
`cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`, `cp_paeth`,
`cp_make32`, `cp_chunk`, `cp_find`.

Types `cp_pixel_t`, `cp_image_t`, `cp_state_t`, `cp_raw_png_t` are translated as
`#[repr(C)]` structs. `cp_state_t` layout fidelity is load-bearing: `cp_decode`
can read `tree[-1]`, which for `tree == s->len` aliases `s->dst[31]` and for
`tree == s->lit` aliases `s->lookup[510..511]`. `#[repr(C)]` reproduces those
offsets exactly.

## Undefined (imported) symbols

C `.so` imports only libc/libm: `__assert_fail`, `calloc`, `free`, `memcmp`,
`memcpy`, `memset` (plus weak `_ITM_*`, `__gmon_start__`, `__cxa_finalize`).
Rust `.so` imports the usual Rust/libc set; **0 missing/undefined non-libc
symbols** — verified with `ldd -r` (no "undefined symbol" lines).

## CRITICAL BUILD FACT: `assert()` is LIVE in the C `.so`

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, and the documented build
command passes only `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`. The actual compile
line is:

```
/usr/bin/cc -D..._EXPORTS -I include -I src -fPIC -MD -MT ... -c src/lib.c
```

There is **no `-DNDEBUG`**, so every `assert()` in `lib.c` is compiled in.
`nm -D --undefined-only` shows `U __assert_fail@GLIBC_2.2.5`, and all nine
assert expression strings are present in `.rodata`:

```
!(s->bits_left & 7)                        (cp_ptr)
s->word_index <= s->word_count             (cp_peak_bits)
s->count >= num_bits_to_read               (cp_consume_bits)
num_bits_to_read <= 32                     (cp_read_bits)
num_bits_to_read >= 0                      (cp_read_bits)
s->bits_left > 0                           (cp_read_bits)
s->count <= 64                             (cp_read_bits)
!cp_would_overflow(s, num_bits_to_read)    (cp_read_bits)
len < 16                                   (cp_build)
(search >> len) == (key >> len)            (cp_decode)
```

A failing assert calls `__assert_fail` → `abort()` → **SIGABRT**. This is
observable behaviour of the ground-truth library and is reachable from trivial
inputs (e.g. `cp_inflate(in, 0, out, n)` trips `assert(s->bits_left > 0)`).
The Rust translation must therefore abort under exactly the same conditions.
See `ERRORS.md` rows E1–E10.
