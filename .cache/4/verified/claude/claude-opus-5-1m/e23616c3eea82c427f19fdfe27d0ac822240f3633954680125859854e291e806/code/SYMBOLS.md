# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

## Source inventory (mechanical)

Every file the C build compiles, per `c_src/CMakeLists.txt`:

```cmake
add_library(SimpleList SHARED
    src/simplestruct.c)
```

* Translation units: `c_src/src/simplestruct.c` — **1 of 1 translated**.
* Public headers: `c_src/include/simplestruct.h` — declares `struct ListNode`
  and `int smallestValue(struct ListNode *)`. Nothing else.
* No other `.c` / `.h` files exist under `c_src/`, so no module was skipped.

```
$ find c_src -type f
c_src/CMakeLists.txt
c_src/include/simplestruct.h
c_src/src/simplestruct.c
```

## `nm -D --defined-only` comparison

Commands used:

```
nm -D --defined-only c_src/build/libSimpleList.so
nm -D --defined-only target/release/libSimpleList.so
```

| # | symbol | type | in C `.so` | in Rust `.so` | Rust definition | status |
|---|--------|------|-----------|---------------|-----------------|--------|
| 1 | `smallestValue` | `T` (global text) | yes | yes | `src/lib.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn smallestValue` | **MATCH** |

Raw output:

```
=== C   === 00000000000010f9 T smallestValue
=== Rust === 0000000000011c40 T smallestValue
```

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libSimpleList.so | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/release/libSimpleList.so | awk '{print $3}' | sort)
(empty)
```

**Missing from Rust: 0. Extra in Rust: 0.** No `#[no_mangle]` wrapper had to be
added and no untranslated C module was found — the diff is empty.

Note: `struct ListNode` contributes no symbol (it is a type, not an object), so
the total exported ABI of the library is the single function above.

## Undefined (imported) symbols

The C `.so` imports only weak CRT hooks
(`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports the same weak CRT hooks plus the Rust standard-library
runtime's dependencies, all of which are **libc / libgcc platform symbols**:
`malloc`, `calloc`, `realloc`, `free`, `posix_memalign`, `memcpy`, `memmove`,
`memset`, `bcmp`, `strlen`, `open64`, `close`, `read`, `write`, `writev`,
`lseek64`, `stat64`, `fstat64`, `statx`, `mmap64`, `munmap`, `getcwd`,
`getenv`, `readlink`, `realpath`, `abort`, `syscall`, `gettid`,
`__errno_location`, `__tls_get_addr`, `__cxa_thread_atexit_impl`,
`dl_iterate_phdr`, `pthread_key_{create,delete}`, `pthread_setspecific`, and
the `_Unwind_*` family (libgcc unwinder, pulled in by the default
`panic = "unwind"` of the dev profile).

**Non-libc undefined symbols in the Rust `.so`: 0.** These imports are an
artifact of linking `std`, not of a missing translation unit; the library
resolves them from `libc.so.6` / `libgcc_s.so.1`, which are already loaded in
any process that `dlopen`s it. Verified by the fact that every differential
test successfully `dlopen`s the Rust `.so` and calls into it.

## Verdict

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined **non-libc** symbols in the Rust `.so`.
- [x] All 1 C translation unit is translated; nothing stubbed or
      `unimplemented!()`.
