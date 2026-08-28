# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-5x6dJI.so   (name = parent dir name, per CMakeLists.txt)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libmerge_sort_lib.so
```

## C `.so` — dynamic symbol table (`nm -D`)

```
                 U memcpy@GLIBC_2.14
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
00000000000012e0 T merge_sort
```

## Defined (`T`) symbol parity

| # | C symbol | kind | in Rust `.so`? | notes |
|---|----------|------|----------------|-------|
| 1 | `merge_sort` | `T` (global text) | **YES** — `T merge_sort` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn merge_sort` in `src/lib.rs` |

`comm -23` of the two defined-symbol name lists is **EMPTY** → 0 missing symbols.

```sh
comm -23 <(nm -D --defined-only c_src/build/libharvest-work-5x6dJI.so   | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only translation/target/release/libmerge_sort_lib.so | awk '{print $NF}' | sort -u)
# (no output)
```

## Non-exported C functions (`static` — correctly absent from BOTH `.so`s)

These are `static` in `c_src/src/lib.c`, so they have no dynamic symbol and are
**not** part of the ABI. They are fully translated in `src/lib.rs` as private
`unsafe fn`s and are exercised *indirectly* through `merge_sort` (see
`CONFIGS.md` for the input shapes that provably reach every branch of each).
Adding `#[no_mangle]` wrappers for them would *add* symbols the C `.so` does not
have, so it is deliberately not done.

| C static function | Rust counterpart | reached from |
|---|---|---|
| `spritebatch_internal_sprite_less_than_or_equal` | `spritebatch_internal_sprite_less_than_or_equal` | `_iteration` |
| `spritebatch_internal_merge_sort_iteration` | `spritebatch_internal_merge_sort_iteration` | `_recurse` |
| `spritebatch_internal_merge_sort_recurse` | `spritebatch_internal_merge_sort_recurse` | `merge_sort` |

No C source file was left untranslated: `c_src/src/lib.c` is the only `.c` file
in `CMakeLists.txt`, and all 4 of its functions are present in `src/lib.rs`.

## Undefined (`U`/`w`) symbols in the Rust `.so`

All are libc / libgcc-unwind imports pulled in by the Rust standard library
runtime (`memcpy`, `malloc`, `free`, `_Unwind_*`, `dl_iterate_phdr`, …).
**0 undefined non-libc symbols.** The C `.so` imports `memcpy` only; the Rust
`.so` imports a superset consisting purely of platform/runtime symbols, which is
expected for a `cdylib` linked against `std` and does not affect the ABI.

## Types

`spritebatch_sprite_t` is a `typedef` — types emit no symbols. Layout parity was
verified independently with `offsetof`/`sizeof` against the C compiler:

| property | C (gcc, x86-64) | Rust `#[repr(C)]` |
|---|---|---|
| `sizeof` | 16 | 16 |
| `_Alignof` | 8 | 8 |
| `offsetof(texture_id)` | 0 | 0 |
| `offsetof(sort_bits)` | 8 | 8 |
| trailing padding | bytes 12..16 | bytes 12..16 |
