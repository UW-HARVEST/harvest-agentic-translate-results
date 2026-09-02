# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D` on both shared libraries.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (completeness check)

`c_src` contains exactly one translation unit, so there is no possibility of a
whole-module omission:

```
c_src/CMakeLists.txt        -> add_library(driver SHARED src/driver.c)
c_src/include/driver.h      -> declares: void driver(const char *in);
c_src/src/driver.c          -> 80 lines, the only .c file
```

Every function in `driver.c` and its linkage:

| C function | linkage | translated to |
|---|---|---|
| `static void add_floor(house_t *)` | internal | private `fn add_floor` |
| `static void add_bedrooms(house_t *, int)` | internal | private `fn add_bedrooms` |
| `static void print_house(house_t *)` | internal | private `fn print_house` |
| `void run(house_t *, int)` | **external** | `#[no_mangle] pub unsafe extern "C" fn run` |
| `static bool parse_val(const char *, int *)` | internal | private `unsafe fn parse_val` |
| `void driver(const char *)` | **external** | `#[no_mangle] pub unsafe extern "C" fn driver` |

There are no macros that generate symbols, no `#ifdef`-gated definitions, and
no other compiled sources. The external-linkage set is therefore exactly
`{driver, run}`.

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `driver` | `T` (0x12c9) | `T` (0x11830) | **match** |
| 2 | `run`    | `T` (0x11c3) | `T` (0x11980) | **match** |

`diff <(nm -D --defined-only C | awk '{print $NF}' | sort) <(… Rust …)` is
**empty**. 0 symbols missing from the Rust `.so`.

Nothing was stubbed: both Rust exports contain the full translated logic
(`run` performs the four `print_house` calls with the mutations interleaved;
`driver` performs `parse_val` plus the two `run` calls, or the error message).

Note: the Rust `.so` additionally exposes no extra `T` symbols of its own —
`house_t` is a `#[repr(C)]` type (no symbol) and the private helpers are not
`#[no_mangle]`.

## Undefined (imported) symbols

All imports of the Rust `.so` are libc / libgcc-unwind symbols; there are **0
missing/undefined non-libc symbols**.

C `.so` imports:

```
w _ITM_deregisterTMCloneTable   w _ITM_registerTMCloneTable   w __cxa_finalize
U __errno_location   w __gmon_start__   U printf   U puts   U strtol
```

Rust `.so` imports (superset; the extra entries are the Rust runtime's
allocator, panic/unwind and std-io plumbing, all resolved by `libc.so.6` and
`libgcc_s.so.1` per `ldd`):

```
_Unwind_*            (libgcc_s.so.1)
__errno_location, __tls_get_addr, __cxa_thread_atexit_impl, abort, bcmp,
calloc, close, dl_iterate_phdr, free, fstat64, getcwd, getenv, gettid,
lseek64, malloc, memcpy, memmove, memset, mmap64, munmap, open64,
posix_memalign, printf, pthread_key_create, pthread_key_delete,
pthread_setspecific, puts, read, readlink, realloc, realpath, stat64, statx,
strlen, strtol, syscall, write, writev          (libc.so.6)
```

Relevant overlap: the Rust translation imports the **same** `printf`,
`strtol` and `__errno_location` from the same `libc.so.6` that the C library
uses. That is what makes byte-identical formatting (`%d`, `%.1f`), identical
`strtol` whitespace/partial-parse/`ERANGE` semantics, and a shared `errno`
and shared `stdout` FILE buffer possible. (The C library's `printf("An error
occurred\n")` was folded by gcc into `puts("An error occurred")`; the emitted
bytes are identical to the Rust `printf` call.)

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the complete
set of feature combinations is the single empty/default one. Verified:

```sh
grep -n '^\[features\]' translation/Cargo.toml   # -> no match
cargo check --no-default-features                # -> ok
```

## Verified state (re-checked after every change)

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $NF}' | sort -u) \
       <(nm -D --defined-only translation/target/debug/libdriver.so   | awk '{print $NF}' | sort -u)
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $NF}' | sort -u) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
```

Both diffs are empty, for both profiles. Enforced as tests
(`tests/symbols.rs::d_exported_symbol_parity`,
`tests/symbols.rs::d_no_unresolved_non_libc_imports`) and by
`translation/phase_d.sh`.

## Note on how the translation preserves the C's *fault* behaviour

`run` and `driver` dereference their pointer arguments with no null check, so an
invalid pointer must fault (SIGSEGV) rather than trap. Rust inserts a
"null pointer dereference" check at every raw-pointer place projection
(`(*p).field`) whenever `-C debug-assertions` is on, which converts that fault
into a SIGABRT panic — an observable difference caught by
`tests/error_path.rs::e16_run_null_pointer`. The translation therefore reads and
writes `house_t` fields through libc `memcpy` at an address computed with
integer arithmetic (`offset_of!`-derived offsets), so no Rust dereference occurs
and the behaviour is identical under every profile. Confirmed by running the
whole suite with `RUSTFLAGS="-C debug-assertions=on -C overflow-checks=on"`.
