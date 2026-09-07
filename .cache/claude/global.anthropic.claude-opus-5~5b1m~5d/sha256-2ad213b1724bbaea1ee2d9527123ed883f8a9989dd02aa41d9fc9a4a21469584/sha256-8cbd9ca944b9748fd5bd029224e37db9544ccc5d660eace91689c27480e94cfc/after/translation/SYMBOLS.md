# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#  -> c_src/build/libdriver.so
cd translation && cargo build --release
#  -> translation/target/release/libdriver.so
```

## Public header surface (`c_src/include/lib.h`)

The header is a single line — the entire public API:

```c
char *searchAndReplace(const char *orig, const char *search, const char *value);
```

There are no namespace/renaming macros, so the linker symbol is literally
`searchAndReplace`. There is no binary/driver executable target in
`c_src/CMakeLists.txt` (only `add_library(driver SHARED src/lib.c)`), so there
is no stdout-comparison step for this project.

## Exported (defined) dynamic symbols

`nm -D --defined-only`

| # | symbol | C `libdriver.so` | Rust `libdriver.so` | status |
|---|--------|------------------|---------------------|--------|
| 1 | `searchAndReplace` | `T` (0x1159) | `T` (0x11730) | ✅ present in both |

Symbol diff (C-exported minus Rust-exported): **EMPTY** — 0 missing symbols.

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $NF}' | sort) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
(no output)
```

No stubs / `unimplemented!()` were introduced: the single symbol is a real,
literal translation of `c_src/src/lib.c`.

## Undefined (imported) dynamic symbols

The C object imports `malloc`, `realloc`, `strdup`, `strlen`, `strncpy`,
`strstr` (all libc), plus the usual weak `_ITM_*` / `__gmon_start__` /
`__cxa_finalize` glue.

The Rust `cdylib` imports `malloc`, `realloc`, `strdup`, `strlen` from libc
directly and additionally pulls in the standard libc/`libgcc` surface used by
the Rust runtime (`memcpy`, `memmove`, `memset`, `bcmp`, `calloc`,
`posix_memalign`, `free`, `abort`, `__errno_location`, `_Unwind_*`,
`pthread_key_*`, `dl_iterate_phdr`, file/`stat` syscall wrappers used by the
panic/backtrace machinery, …).

**0 missing / undefined non-libc symbols in the Rust `.so`.** `strncpy` and
`strstr` are not imported because the translation reimplements their exact
semantics in-crate (`c_strncpy`, `c_strstr`) — this is an implementation detail,
not an API difference; the exported surface is identical.

Note: the Rust crate deliberately allocates the returned buffer with the **C**
allocator (`malloc`/`realloc`/`strdup`) so callers can release it with `free()`,
exactly as with the C library. This is why those four libc symbols are imported.

## Symbol parity across build configurations

`cargo metadata` reports an empty `[features]` table for this package, so there
is exactly **one** feature combination. `run_all_configs.sh` nevertheless builds
and re-checks parity for `{dev, release} × {default, --no-default-features,
--all-features}` — six configurations — because the two cargo *profiles* really
are different code (`[profile.release] panic = "abort"` vs. the dev profile's
unwinding panics plus debug assertions / overflow checks). See
`all_configs.log`: `symbol parity: OK (0 missing)` for all six.
