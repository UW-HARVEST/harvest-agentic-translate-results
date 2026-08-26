# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects. No assumptions.

## Artifacts under comparison

| side | path | build command |
|------|------|---------------|
| C    | `c_src/build/libtranslated_rust.so` | `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .` |
| Rust | `target/debug/libmerge_sort_lib.so`  | `cargo build` (crate-type = `cdylib`) |

## Reproduce

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | grep -v '^$' | sort > /tmp/c_syms.txt
nm -D --defined-only target/debug/libmerge_sort_lib.so | awk '{print $3}' | grep -v '^$' | sort > /tmp/rust_syms.txt
comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt   # MUST be empty
```

## Defined dynamic symbols exported by the C `.so`

`nm -D --defined-only c_src/build/libtranslated_rust.so`:

```
00000000000012e0 T merge_sort
```

That is the complete list: **1 symbol**. The C translation unit declares every
helper `static`, so `spritebatch_internal_sprite_less_than_or_equal`,
`spritebatch_internal_merge_sort_iteration` and
`spritebatch_internal_merge_sort_recurse` have internal linkage and are
deliberately absent from the dynamic symbol table. Confirmed:

```sh
$ nm -D c_src/build/libtranslated_rust.so | grep -c spritebatch_internal
0
```

## Defined dynamic symbols exported by the Rust `.so`

`nm -D --defined-only target/debug/libmerge_sort_lib.so`:

```
0000000000012b00 T merge_sort
```

## Parity table

| # | C symbol | type | present in Rust `.so`? | Rust item providing it |
|---|----------|------|------------------------|------------------------|
| 1 | `merge_sort` | `T` (global text) | **YES** — exact name | `#[unsafe(no_mangle)] pub unsafe extern "C" fn merge_sort` in `src/lib.rs` |

**Symbols missing from Rust: 0.** `comm -23` output is empty.

No symbol required translation of a skipped C module: `c_src/src/lib.c` is the
only C source file listed in `c_src/CMakeLists.txt`, and all four of its
functions (1 public + 3 `static`) have counterparts in `src/lib.rs`. Nothing is
stubbed or `unimplemented!()`.

## Internal (non-exported) function parity

Not part of the ABI, but tracked for completeness — these must exist so the
behaviour reachable through `merge_sort` is complete:

| C `static` function | Rust counterpart | present |
|---------------------|------------------|---------|
| `spritebatch_internal_sprite_less_than_or_equal` | `fn spritebatch_internal_sprite_less_than_or_equal` | YES |
| `spritebatch_internal_merge_sort_iteration` | `unsafe fn spritebatch_internal_merge_sort_iteration` | YES |
| `spritebatch_internal_merge_sort_recurse` | `unsafe fn spritebatch_internal_merge_sort_recurse` | YES |

Because these are `static` in C they are **not callable across the FFI
boundary on either side**. They are therefore exercised *indirectly* through
`merge_sort`; `CONFIGS.md` records which row forces which internal branch.

## Undefined symbols in the Rust `.so`

All undefined symbols are libc / libgcc-unwind imports pulled in by the Rust
standard library (`memcpy`, `malloc`, `_Unwind_*`, `__cxa_finalize`, …). There
are **0 undefined non-libc symbols**, i.e. no unresolved references to
untranslated code.

## ABI / layout parity

`spritebatch_sprite_t` verified against the C compiler:

```
size=16 align=8 off_tex=0 off_sb=8      # gcc -I c_src/include, __builtin_offsetof
```

The Rust `#[repr(C)] struct spritebatch_sprite_t { texture_id: u64, sort_bits: c_int }`
has identical size (16), alignment (8) and field offsets (0, 8), leaving the
same 4 bytes of tail padding at offsets 12..16. This is asserted at runtime by
the `abi_layout_matches_c` test.
