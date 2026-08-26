# SYMBOLS.md — Symbol parity between C `.so` and Rust `.so`

Derived mechanically from `nm -D` on both shared libraries. No assumptions.

## How the artifacts were produced

```sh
# C
mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#   -> c_src/build/libdriver.so

# Rust
cargo build --offline --release            # no [features] exist; single configuration
#   -> target/release/libdriver.so
```

```sh
nm -D --defined-only c_src/build/libdriver.so   | awk '{print $3}' | sort -u > c_syms.txt
nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u > rs_syms.txt
comm -23 c_syms.txt rs_syms.txt   # missing from Rust
comm -13 c_syms.txt rs_syms.txt   # extra in Rust
```

## Complete C source surface

The whole library is **one translation unit** (`c_src/src/driver.c`) exposing
**one** public function via `c_src/include/driver.h`:

```c
void driver(const char *s1, const char *s2);
```

Body (verbatim, the only non-comment code in the library):

```c
void driver(const char *s1, const char *s2) {
    printf("%zu\n", strcspn(s1, s2));
}
```

`CMakeLists.txt` compiles exactly `src/driver.c` into `libdriver.so`. There is
no second module, no conditionally compiled file, and no macro-generated
symbol, so there is no C source that could have been left untranslated.

## Exported (defined, dynamic) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|-----------|-------|
| 1 | `driver` | `T` (text, global) | `T` (text, global) | `#[no_mangle] pub unsafe extern "C" fn driver` |

* C defined dynamic symbols: **1**
* Rust defined dynamic symbols: **1**
* Missing from Rust (`comm -23`): **none** (empty)
* Extra in Rust (`comm -13`): **none** (empty)

**Symbol diff is EMPTY in both directions.** No `#[no_mangle]` wrapper had to be
added and no untranslated C module was discovered.

## Undefined symbols (imports)

All Rust imports resolve against the platform C runtime; there are **0 missing /
unresolvable non-libc symbols**. Verified with `ldd` (no "not found" entries):

```
linux-vdso.so.1
libgcc_s.so.1 => /lib64/libgcc_s.so.1
libc.so.6     => /lib64/libc.so.6
/lib64/ld-linux-x86-64.so.2
```

| library | undefined symbols | all resolvable? |
|---------|-------------------|-----------------|
| C `.so` | `printf`, `strcspn` (+ weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`) | yes — glibc |
| Rust `.so` | `printf`, `strlen`, `memcpy`, `malloc`, `free`, `realloc`, `calloc`, `posix_memalign`, `memset`, `memmove`, `bcmp`, `abort`, `__errno_location`, `open64`, `read`, `close`, `write`, `writev`, `lseek64`, `stat64`, `fstat64`, `mmap64`, `munmap`, `getcwd`, `getenv`, `readlink`, `realpath`, `syscall`, `dl_iterate_phdr`, `pthread_key_*`, `pthread_setspecific`, `__tls_get_addr`, `_Unwind_*` (+ weak `statx`, `gettid`, `__cxa_*`, `_ITM_*`, `__gmon_start__`) | yes — glibc + libgcc_s |

The Rust `.so`'s extra imports are the Rust standard library's runtime
(allocator, panic/unwind machinery, std file/`env` support) — not application
symbols. The Rust translation calls the *same* `printf@GLIBC_2.2.5` the C
library calls, so number formatting, stream selection and buffering are
identical by construction. `strcspn` is reimplemented in Rust (see
`ERRORS.md` / `CONFIGS.md` for the differential verification of its semantics).

## Completion gate

Re-checked automatically on every run of `./run_all_tests.sh`, for **both** the
`release` and the `debug` Rust artifact:

- [x] `nm -D` shows **0 missing** symbols in the Rust `.so` (C→Rust diff empty).
- [x] `nm -D` shows **0 extra** exported symbols in the Rust `.so`.
- [x] `nm -D` shows **0 unresolvable non-libc undefined** symbols in the Rust
      `.so` (`ldd -r` reports no "not found" / "undefined symbol").
- [x] No C source file/module was left untranslated (single-TU library), so no
      symbol needed a new `#[no_mangle]` wrapper and nothing was stubbed.

Latest run:

```
C .so exports 1 symbol(s): driver
[PASS] release: 0 missing symbols (all 1 C symbols exported by Rust)
[PASS] release: 0 unresolvable undefined symbols
[PASS] debug:   0 missing symbols (all 1 C symbols exported by Rust)
[PASS] debug:   0 unresolvable undefined symbols
```

All differential tests reach `driver` by `dlopen`+`dlsym` on the two `.so` files
(`libloading`), never by a direct Rust call, so the `#[no_mangle] extern "C"`
export wrapper is itself under test. `Harness::new` asserts the two resolved
`driver` addresses differ, which prevents a `dlsym` scope collapse from making
every comparison vacuously pass.
