# SYMBOLS.md — Phase A symbol surface

## Source of truth

C shared library: `c_src/build/libharvest-work-pgHR6S.so` (built from the single
translation unit `c_src/src/lib.c`).
Rust shared library: `translation/target/release/libmerge_sort_lib.so`
(`crate-type = ["cdylib"]`).

## `nm -D --defined-only` on the C `.so`

```
00000000000012e0 T merge_sort
```

## `nm -D --defined-only` on the Rust `.so`

```
00000000000117f0 T merge_sort
```

## Parity table

| # | C symbol | type | exported by Rust `.so`? | notes |
|---|----------|------|-------------------------|-------|
| 1 | `merge_sort` | `T` (global text) | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn merge_sort` | only public symbol of the library |

### Missing symbols

**None.** The symbol diff is EMPTY in both directions (no C symbol absent from
Rust, no extra non-boilerplate symbol in Rust).

### `static` C functions (deliberately NOT exported)

These are `static` in `c_src/src/lib.c`, so they have no dynamic symbol in the C
`.so` and must NOT be exported by the Rust `.so` either. All three ARE
translated (as private `unsafe fn`s), so this is not a completeness gap:

| C `static` function | Rust counterpart | exported? (correct) |
|---------------------|------------------|---------------------|
| `spritebatch_internal_sprite_less_than_or_equal` | `spritebatch_internal_sprite_less_than_or_equal` | no (matches C) |
| `spritebatch_internal_merge_sort_iteration` | `spritebatch_internal_merge_sort_iteration` | no (matches C) |
| `spritebatch_internal_merge_sort_recurse` | `spritebatch_internal_merge_sort_recurse` | no (matches C) |

### Undefined (imported) symbols

C `.so` imports: `memcpy@GLIBC_2.14` plus the usual weak CRT stubs
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`).
The Rust `.so` imports only libc/CRT symbols (`memcpy`, unwinder/allocator
stubs). **0 missing/undefined non-libc symbols.**

### Data types crossing the FFI boundary

| C | Rust | size / align (x86-64) |
|---|------|------------------------|
| `struct spritebatch_sprite_t { unsigned long long texture_id; int sort_bits; }` | `#[repr(C)] struct spritebatch_sprite_t { texture_id: c_ulonglong, sort_bits: c_int }` | 16 bytes, align 8, 4 trailing padding bytes |

## Automated checks

Symbol parity is not just recorded here, it is asserted by tests
(`tests/phase_d_symbols.rs`) and by `verify.sh`:

| check | test |
|---|---|
| every C dynamic symbol is exported by the Rust `.so` under the same name | `every_c_symbol_is_exported_by_rust` |
| the three `static` C helpers are exported by neither | `c_static_helpers_are_not_exported_by_either` |
| the Rust `.so` has no unresolved non-libc symbols (each undefined symbol is proven resolvable via `dlsym(RTLD_DEFAULT, ..)`) | `rust_so_has_no_unresolved_non_libc_symbols` |
| the Rust `.so` publishes no extra public API beyond the C's | `rust_so_exports_no_extra_public_api` |
| `comm -23` of the two `nm -D --defined-only` outputs is empty, for every feature combination | `verify.sh` |

All pass. The symbol diff is empty in both directions.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one (`--no-default-features` is equivalent).
See `CONFIGS.md` for the runtime configuration surface, which is where all the
real variation lives.
