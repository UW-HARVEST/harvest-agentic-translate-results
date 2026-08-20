# SYMBOLS.md — public symbol parity (Phase A / Phase D)

## What is being compared

`c_src/CMakeLists.txt` builds an **executable** (`add_executable(driver src/main.c)`),
so the project has no library target of its own. To be able to compare the two
implementations *through the FFI boundary* (as required), the single C
translation unit is additionally compiled as a shared object with the same
(default, unoptimised) flags CMake uses:

```sh
# c_src is never modified; the object is written to ./c_build
gcc -fPIC -shared -o c_build/libcdriver.so c_src/src/main.c
```

The Rust crate mirrors this: `src/imp.rs` holds the translation, `src/main.rs`
is the `driver` executable, and `src/lib.rs` is a `cdylib`
(`target/<profile>/libdriver.so`) that exports the *same* C ABI symbols as the C
translation unit.

Both `.so`s are loaded with `libloading` in the tests; the Rust implementation is
**never** called directly, always through `dlsym` on `libdriver.so`.

## `nm -D --defined-only` (dynamic, defined)

| # | symbol | C signature | C `.so` | Rust `.so` | status |
|---|--------|-------------|---------|------------|--------|
| 1 | `run`  | `void run(house_t *the_house, int extra_bedrooms)` | `T run` | `T run` | ✅ exported by both |
| 2 | `main` | `int main(void)` | `T main` | `T main` | ✅ exported by both |

Raw output:

```
$ nm -D --defined-only c_build/libcdriver.so
00000000000012d9 T main
00000000000011d3 T run

$ nm -D --defined-only target/debug/libdriver.so
000000000001bda0 T main
000000000001bdc0 T run
```

**Symbol diff (C minus Rust): EMPTY.** No symbol of the C `.so` is missing from
the Rust `.so`, and the Rust `.so` exports no extra symbols.

### Why there are only two

Everything else in `c_src/src/main.c` has internal linkage and is therefore not
part of the dynamic surface; it is reached (and thus differentially tested)
through `run` and `main`:

| C function | linkage | reached through |
|---|---|---|
| `static void add_floor(house_t *)` | internal | `run` |
| `static void add_bedrooms(house_t *, int)` | internal | `run` |
| `static void print_house(house_t *)` | internal | `run` (4 calls per invocation) |
| `static bool parse_val(const char *, int *)` | internal | `main` |
| `void run(house_t *, int)` | **external** | exported |
| `int main(void)` | **external** | exported |

No macro-generated symbols exist in this translation unit (no `#define` creates
a function or object), and there are no exported data objects.

## Undefined (imported) symbols

The Rust `.so` must not have undefined symbols beyond libc / libgcc_s.

```
$ nm -D --undefined-only c_build/libcdriver.so
_ITM_deregisterTMCloneTable(w) _ITM_registerTMCloneTable(w) __cxa_finalize(w)
__errno_location fgets printf puts stdin strtol __gmon_start__(w)

$ nm -D --undefined-only target/debug/libdriver.so
_Unwind_* (libgcc_s), __cxa_finalize(w), __cxa_thread_atexit_impl(w),
__errno_location, __gmon_start__(w), __tls_get_addr, abort, bcmp, calloc,
close, dl_iterate_phdr, free, fstat64, getcwd, getenv, gettid(w), lseek64,
malloc, memcpy, memmove, memset, mmap64, munmap, open64, posix_memalign,
pthread_key_create, pthread_key_delete, pthread_setspecific, read, readlink,
realloc, realpath, stat64, statx(w), strlen, syscall, write, writev
```

Every undefined symbol in the Rust `.so` resolves to `libc.so.6` /
`libgcc_s.so.1` (the Rust standard library's normal imports).
**0 missing/undefined non-libc symbols.**

Verified mechanically by `./check_symbols.sh` (see that script; it fails if the
defined-symbol sets differ or if a non-libc symbol is unresolved via `ldd -r`).

## Executable-level parity

The delivered artefact is an executable, so the process-level surface is also
compared: `c_src/build/driver` vs `target/<profile>/driver` — same stdin, and
stdout / stderr / exit status / terminating signal compared byte for byte
(`tests/cli_diff.rs`).
