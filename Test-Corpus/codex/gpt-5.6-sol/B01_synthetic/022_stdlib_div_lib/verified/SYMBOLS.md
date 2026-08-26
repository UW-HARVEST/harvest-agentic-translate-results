# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver.so`

Command: `nm -D c_src/build/libdriver.so`

## Defined public symbols

| symbol | C type | Rust type | status |
|--------|--------|-----------|--------|
| `driver` | `T` | `T` | [x] |

## Dynamic dependencies

These are undefined imports, not library exports. All non-weak imports are
provided by libc.

| symbol | C type | provider |
|--------|--------|----------|
| `_ITM_deregisterTMCloneTable` | `w` | toolchain runtime (optional weak import) |
| `_ITM_registerTMCloneTable` | `w` | toolchain runtime (optional weak import) |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc/toolchain runtime |
| `__gmon_start__` | `w` | toolchain runtime (optional weak import) |
| `div@GLIBC_2.2.5` | `U` | libc |
| `printf@GLIBC_2.2.5` | `U` | libc |

The Rust cdylib also imports the standard `libgcc_s` unwinder used by Rust's
runtime. `ldd -r` reports no unresolved relocation in either library. No
application/library implementation dependency is undefined.
