# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## Public (defined, global) symbols exported by the C `.so`

| # | symbol | type | source | exported by Rust `.so`? |
|---|--------|------|--------|--------------------------|
| 1 | `driver` | `T` (text, global) | `c_src/src/driver.c:31`, declared `c_src/include/driver.h:27` | YES — `translation/src/lib.rs`, `#[unsafe(no_mangle)] pub extern "C" fn driver(c: c_char)` |

That is the complete public ABI. `c_src/include/driver.h` declares exactly one
function and there are no namespace-renaming preprocessor macros, so the linker
symbol is the plain name `driver`.

## Symbol diff

C-exported symbols missing from the Rust `.so`: **none** (diff is empty).

No symbol required translation of a skipped module and no symbol is stubbed:
`driver` is fully implemented in Rust (`lib.rs` + `ctype.rs`, the latter
reproducing the glibc `<ctype.h>` table lookups that the C macros expand to).

## Undefined (imported) symbols

The Rust `.so` imports only libc / runtime symbols. The two imports that carry
observable behaviour are the same C-runtime entry points the C library uses:

| symbol | used by C | used by Rust |
|--------|-----------|--------------|
| `printf` | yes (14 calls) | yes (14 calls, same format strings) |
| `setlocale` | yes (`LC_ALL`, `"C"`) | yes (`LC_ALL` = 6, `"C"`) |

The C library resolves the `is*()` / `tolower` / `toupper` calls through glibc's
`<ctype.h>` macros, which read `__ctype_b_loc()` / `__ctype_tolower_loc()` /
`__ctype_toupper_loc()` tables inline rather than calling exported functions;
the Rust port reproduces those tables in `ctype.rs`, so it has no corresponding
imports. This is an implementation detail, not an ABI difference — verified
observationally by the differential tests, which compare the printed bytes.

## Checklist

- [x] `nm -D` shows 0 missing symbols in the Rust `.so` (`driver` present).
- [x] 0 undefined non-libc symbols in the Rust `.so`.
- [x] No stubs / `unimplemented!()` standing in for real behaviour.
