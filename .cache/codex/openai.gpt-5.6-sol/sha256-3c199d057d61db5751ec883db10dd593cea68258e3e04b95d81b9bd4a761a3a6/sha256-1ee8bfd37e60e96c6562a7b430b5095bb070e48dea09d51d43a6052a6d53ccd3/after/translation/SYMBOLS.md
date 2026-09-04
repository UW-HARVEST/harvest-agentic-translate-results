# Dynamic symbol surface

Source library: `../c_src/build/libdriver.so`

Command used:

```text
nm -D ../c_src/build/libdriver.so
```

## Defined public exports

| C symbol | C type | Rust export present | Implementation |
|----------|--------|---------------------|----------------|
| `w_utf8_drop` | `T` | yes | `src/lib.rs` |
| `w_utf8_filter` | `T` | yes | `src/lib.rs` |

Missing Rust exports: **0**

## Imported/weak dynamic symbols reported by `nm -D`

These are recorded so the complete C `nm -D` surface is represented. They are
runtime/toolchain dependencies, not functions exported by this library.

| Symbol | C type | Classification |
|--------|--------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` | toolchain weak import |
| `_ITM_registerTMCloneTable` | `w` | toolchain weak import |
| `__assert_fail@GLIBC_2.2.5` | `U` | libc import |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc/toolchain weak import |
| `__gmon_start__` | `w` | toolchain weak import |
| `malloc@GLIBC_2.2.5` | `U` | libc import |
| `memcpy@GLIBC_2.14` | `U` | libc import |
| `realloc@GLIBC_2.2.5` | `U` | libc import |
| `strdup@GLIBC_2.2.5` | `U` | libc import |
| `strlen@GLIBC_2.2.5` | `U` | libc import |

## Completion check

- [x] Rebuilt C and Rust release libraries have an empty defined-export diff.
- [x] `ldd -r target/release/libdriver.so` reports no unresolved relocation.
