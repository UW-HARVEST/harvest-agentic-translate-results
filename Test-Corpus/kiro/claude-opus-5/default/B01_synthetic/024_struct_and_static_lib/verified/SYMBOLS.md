# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D c_src/build/libdriver.so
nm -D translation/target/release/libdriver.so
```

## C `.so` dynamic symbol table (verbatim)

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
000000000000120f T driver
                 U printf@GLIBC_2.2.5
000000000000119f T run
```

## Defined (exported) symbols — the parity requirement

| # | C symbol | C type | in Rust `.so`? | Rust type | notes |
|---|----------|--------|----------------|-----------|-------|
| 1 | `driver` | `T` (global text) | YES | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver(x: c_int)` |
| 2 | `run`    | `T` (global text) | YES | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn run(extra_bedrooms: c_int)` — not declared in `driver.h`, but non-`static` in `driver.c`, therefore exported; Rust must export it too |

`w` (weak) symbols `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__` are toolchain/CRT artifacts, not library API.
The Rust `.so` also carries all four (verified with `nm -D`).

### C symbols NOT exported (internal, `static` in `driver.c`)

These have no dynamic symbol and MUST NOT be exported by Rust either. They are
reachable only through `run` / `driver`:

| C internal | Rust counterpart | exported by Rust `.so`? |
|---|---|---|
| `static house_t the_house` | `static THE_HOUSE: Global` | no (checked: absent from `nm -D`) |
| `static void add_floor(house_t*)` | `unsafe fn add_floor` | no |
| `static void add_bedrooms(house_t*, int)` | `unsafe fn add_bedrooms` | no |
| `static void add_floor_to_the_house()` | `unsafe fn add_floor_to_the_house` | no |
| `static void print_the_house()` | `unsafe fn print_the_house` | no |

## Undefined-symbol check on the Rust `.so`

`nm -D --undefined-only translation/target/release/libdriver.so` yields only
libc/`glibc` and `libgcc` unwinder imports:

```
_Unwind_* (GCC_*), __cxa_finalize, __cxa_thread_atexit_impl, __errno_location,
__tls_get_addr, abort, bcmp, calloc, close, dl_iterate_phdr, free, fstat64,
getcwd, getenv, gettid, lseek64, malloc, memcpy, memmove, memset, mmap64,
munmap, open64, posix_memalign, printf, pthread_key_create, pthread_key_delete,
pthread_setspecific, read, readlink, realloc, realpath, stat64, statx, strlen,
syscall, write, writev, _ITM_*, __gmon_start__
```

`printf@GLIBC_2.2.5` is imported by BOTH libraries — the Rust translation calls
glibc `printf` directly rather than reimplementing `%d` / `%.1f` formatting, so
number formatting and stdout buffering are identical by construction.

## Result

- Missing from Rust `.so`: **0**
- Undefined non-libc / non-unwinder symbols in Rust `.so`: **0**
- Extra API symbols exported by Rust that C does not export: **0**

No missing implementation and no missing module: `c_src` contains exactly one
translation unit (`src/driver.c`, 66 lines) and it is fully translated in
`translation/src/lib.rs`. No stubs were added.

## No binary target

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/driver.c)`.
There is no `main()` anywhere in `c_src`, and `translation/Cargo.toml` declares
only `crate-type = ["cdylib"]` with no `[[bin]]`. The "compare binary stdout"
requirement is therefore not applicable; stdout is instead compared through the
`.so` boundary by capturing fd 1 around each FFI call.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** and no optional
dependencies, so the only build configuration is the default one. Verified:

```
$ grep -c '\[features\]' translation/Cargo.toml
0
```

`--no-default-features` is therefore identical to the default build; both are
exercised (see `run_all_features.sh`).
