# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

Commands used:

```
nm -D --defined-only c_src/build/libdriver.so       | grep -v ' [a-z] '
nm -D --defined-only translation/target/release/libdriver.so | grep -v ' [a-z] '
```

## C `.so` exported (global/defined) symbols

| # | symbol | type | source of truth |
|---|--------|------|-----------------|
| 1 | `decode_base64` | `T` (text/global) | `c_src/include/lib.h:1`, `c_src/src/lib.c:44` |

The only entry in `c_src/include/lib.h` is:

```c
char *decode_base64(const char *src);
```

`decode()` and `is_base64()` in `c_src/src/lib.c` are `static` — they have
internal linkage and are **not** part of the exported surface (confirmed: they
do not appear in `nm -D --defined-only`). They are therefore correctly kept
private (`fn`, no `#[no_mangle]`) in the Rust translation and are exercised
indirectly through `decode_base64`.

## Rust `.so` exported symbols

| # | symbol | type | Rust definition |
|---|--------|------|-----------------|
| 1 | `decode_base64` | `T` (text/global) | `translation/src/lib.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn decode_base64` |

## Diff

```
comm -3 <(nm -D --defined-only c_src/build/libdriver.so | grep -v ' [a-z] ' | awk '{print $3}' | sort) \
        <(nm -D --defined-only translation/target/release/libdriver.so | grep -v ' [a-z] ' | awk '{print $3}' | sort)
```

Result: **EMPTY** — 0 symbols missing from the Rust `.so`.

No C source file was left untranslated: `c_src/src/lib.c` is the only `.c` file
referenced by `c_src/CMakeLists.txt` (`add_library(driver SHARED src/lib.c)`),
and all three of its functions (`decode`, `is_base64`, `decode_base64`) are
present in `translation/src/lib.rs`. No stubs, no `unimplemented!()`.

## Undefined (imported) symbols

The C `.so` imports `calloc`, `malloc`, `free`, `strlen` from libc. The Rust
`.so` declares and imports the same four libc symbols (`extern "C"` block in
`src/lib.rs`) rather than using Rust's global allocator — this matters because
the caller of `decode_base64` is expected to `free()` the returned pointer.

0 missing/undefined non-libc symbols in the Rust `.so`.
