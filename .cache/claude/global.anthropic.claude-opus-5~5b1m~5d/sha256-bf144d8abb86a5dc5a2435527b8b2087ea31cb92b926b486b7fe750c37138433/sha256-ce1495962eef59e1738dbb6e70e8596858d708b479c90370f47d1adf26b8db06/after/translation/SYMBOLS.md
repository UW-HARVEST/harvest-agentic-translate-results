# SYMBOLS.md — Phase A symbol map

Derived mechanically from `nm -D` on both shared objects. No symbol on this
page was chosen by judgement; the lists below are the raw tool output.

## How the artifacts were produced

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust
cd translation && cargo build --release      # -> target/release/libdriver.so
cd translation && cargo build                # -> target/debug/libdriver.so
```

## C source inventory (completeness check)

The whole library is three files, and every one of them is accounted for in the
Rust crate — there is no untranslated module:

| C file | contents | translated in |
|---|---|---|
| `c_src/include/lib.h` | 1 line: the `tool_basename` declaration | `translation/src/lib.rs` (signature) |
| `c_src/src/lib.c` | 22 lines: the single definition of `tool_basename` | `translation/src/lib.rs` (`tool_basename`, `strrchr_index`) |
| `c_src/CMakeLists.txt` | build recipe, one `add_library(driver SHARED src/lib.c)` | `translation/Cargo.toml` (`crate-type = ["cdylib"]`, `name = "driver"`) |

`grep -c 'return' c_src/src/lib.c` → `1`. There is exactly one function in the
library, so no `#[no_mangle]` wrapper can be missing for a translated-but-
unexported implementation, and no C file was skipped.

## Exported (defined) dynamic symbols

`nm -D --defined-only`:

| symbol | C `libdriver.so` | Rust `libdriver.so` | status |
|---|---|---|---|
| `tool_basename` | `T` (0x1109) | `T` (0x11c50) | ✅ present in both |

Raw output:

```
=== C ===
0000000000001109 T tool_basename
=== Rust ===
0000000000011c50 T tool_basename
```

Name-only diff (this is the gate from Phase D):

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so           | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
# -> empty (exit 0)
```

**0 symbols missing from the Rust `.so`. 0 extra symbols. Symbol diff is empty.**

This diff is also asserted automatically by
`translation/tests/phase_d_symbols.rs`, so it cannot silently regress.

## Undefined (imported) symbols

The C object imports one non-weak symbol; the Rust object imports the Rust
standard library's usual libc/unwinder set. All are libc, `libgcc_s`
(`_Unwind_*`) or glibc-weak stubs — i.e. resolved by the platform, not missing
pieces of this library.

| | C | Rust |
|---|---|---|
| non-libc undefined symbols | none | none |
| libc / platform undefined | `strrchr`, `__cxa_finalize` (w), `__gmon_start__` (w), `_ITM_*` (w) | `strlen`, `memcpy`, `malloc`, `free`, `realloc`, `calloc`, `posix_memalign`, `memmove`, `memset`, `bcmp`, `abort`, `getenv`, `getcwd`, `readlink`, `realpath`, `open64`, `close`, `read`, `write`, `writev`, `lseek64`, `stat64`, `fstat64`, `statx` (w), `mmap64`, `munmap`, `dl_iterate_phdr`, `syscall`, `gettid` (w), `__errno_location`, `__tls_get_addr`, `pthread_key_{create,delete}`, `pthread_setspecific`, `__cxa_thread_atexit_impl` (w), `__cxa_finalize` (w), `__gmon_start__` (w), `_ITM_*` (w), `_Unwind_*` |

The Rust import list is larger only because `libstd` is linked in (panic
machinery, backtrace support, allocator shims). None of it is a dangling
reference to an untranslated part of `libdriver`.

**Verdict: `nm -D` shows 0 missing and 0 undefined non-libc symbols in the Rust
`.so`. Phase A symbol requirement satisfied.**
