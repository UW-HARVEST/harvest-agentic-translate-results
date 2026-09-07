# SYMBOLS.md — dynamic symbol parity (Phase A / Phase D)

## Method

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libharvest-work-7Yz9J8.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libima_parse_lib.so
```

## C source surface

The C library is a single translation unit (`c_src/src/lib.c`) with one public
header (`c_src/include/lib.h`).

`c_src/include/lib.h` declares exactly one function:

* `int ima_parse(struct ima_info *info, const void *data);`

Everything else in `c_src/src/lib.c` is declared `static` (internal linkage) and
therefore is **not** part of the ABI:

| C entity | linkage | in `.so` dynsym? |
|----------|---------|------------------|
| `ima_bswap16` | `static` | no |
| `ima_bswap32` | `static` | no |
| `ima_bswap64` | `static` | no |
| `ima_btoh16`  | `static` | no |
| `ima_btoh32`  | `static` | no |
| `ima_btoh64`  | `static` | no |
| `ima_parse`   | external | **yes** |

No macros in the C source generate additional exported symbols. There are no
exported data objects. `CMakeLists.txt` adds no `-D` defines, no visibility
attributes and no other source files, so `src/lib.c` is the whole library.

## `nm -D --defined-only` — C `.so` (ground truth)

| # | symbol | type |
|---|--------|------|
| 1 | `ima_parse` | `T` |

(Total: 1 defined dynamic symbol. Everything else in the C `.so` dynsym table is
undefined/`w` linker-supplied boilerplate: `_ITM_deregisterTMCloneTable`,
`__gmon_start__`, `_ITM_registerTMCloneTable`, `__cxa_finalize` — these come
from the toolchain, not from the source, and are not part of the library API.)

## `nm -D --defined-only` — Rust `.so`

| # | symbol | type |
|---|--------|------|
| 1 | `ima_parse` | `T` |

## Diff

```
$ diff <(nm -D --defined-only <c.so>  | awk '{print $3}' | sort) \
       <(nm -D --defined-only <rs.so> | awk '{print $3}' | sort)
(no output)
```

**Result: EMPTY diff. 0 symbols missing from the Rust `.so`.**

No C module was skipped by the translation: `src/lib.c` is the only C source
file and all of its contents (the six `static` byte-swap helpers plus
`ima_parse`) are present in `translation/src/lib.rs`. The helpers are `fn`
(not `#[no_mangle]`), which is correct — they are `static` in C and must **not**
be exported.

## Undefined-symbol check for the Rust `.so`

```
$ nm -D --undefined-only translation/target/release/libima_parse_lib.so
```

Only libc/toolchain-supplied symbols appear (no unresolved crate-level
symbols). 0 missing non-libc symbols.

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
