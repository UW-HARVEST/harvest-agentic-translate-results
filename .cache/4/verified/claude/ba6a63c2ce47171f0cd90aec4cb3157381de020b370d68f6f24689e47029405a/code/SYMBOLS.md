# SYMBOLS.md — public symbol parity (Phase A / Phase D)

## How this table was produced

```sh
# C ground truth, built exactly as CMakeLists.txt does (default CMAKE_BUILD_TYPE,
# -DCMAKE_POSITION_INDEPENDENT_CODE=ON) but linked -shared so it can be dlopen'd:
cc -shared -fPIC -o libcdriver.so c_src/src/container_of.c
nm -D libcdriver.so

# Rust translation (crate-type = ["cdylib"]):
cargo build
nm -D target/debug/libdriver.so
```

`build.rs` performs the C compilation automatically into `OUT_DIR` and publishes
the result as the `C_SO_PATH` env var, so `cargo test` is self-contained.
Nothing in `c_src/` is modified.

## Translation-unit inventory

`c_src/` contains exactly one translation unit, and all of it is translated —
there is no skipped module:

| C file | lines | translated to |
|--------|-------|---------------|
| `c_src/src/container_of.c` | 33 | `src/container_of.rs` (logic), `src/lib.rs` (C-ABI exports), `src/main.rs` (`driver` executable) |

Macros in the C file (`offsetof`, `container_of`) generate no symbols of their
own; they are expanded inside the two `find_container_of_*` functions and are
modelled by `container_of::OFFSET_OF_A` / `OFFSET_OF_B` (via
`core::mem::offset_of!` on a `#[repr(C)]` struct) plus the wrapping pointer
subtraction in `find_container_of_a` / `find_container_of_b`.

## DEFINED (exported) symbols — must match exactly

| # | symbol | C `nm -D` | Rust `nm -D` | C declaration | Rust export site | status |
|---|--------|-----------|--------------|---------------|------------------|--------|
| 1 | `find_container_of_a` | `T` | `T` | `struct test *find_container_of_a(int *i)` | `src/lib.rs` `#[no_mangle] extern "C"` | ✅ present |
| 2 | `find_container_of_b` | `T` | `T` | `struct test *find_container_of_b(int *i)` | `src/lib.rs` `#[no_mangle] extern "C"` | ✅ present |
| 3 | `main` | `T` | `T` | `int main(int argc, char **argv)` | `src/lib.rs` `#[no_mangle] extern "C"` | ✅ present |

`main` is exported from the `cdylib` on purpose: the C shared object exports it,
so symbol parity requires it, and it lets the differential tests drive the entire
program body (argument parsing → `container_of` round trip → `printf`) across the
FFI boundary instead of only the two leaf helpers.

**Missing-symbol count: 0.** No stubs, no `unimplemented!()`, no fake exports —
each of the three symbols forwards to the real translated logic.

## Linker/toolchain-generated symbols (not part of the API)

Present as *weak/undefined* in both objects; not API surface, no action needed:

| symbol | C | Rust |
|--------|---|------|
| `_ITM_deregisterTMCloneTable` | `w` | `w` |
| `_ITM_registerTMCloneTable` | `w` | `w` |
| `__gmon_start__` | `w` | `w` |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | `w` |

## UNDEFINED (imported) symbols

The undefined sets are *not* required to match: they are each implementation's
private choice of libc/runtime helpers. What matters is that the Rust object has
**no undefined non-libc / non-runtime symbol**.

C imports: `atoi@GLIBC_2.2.5`, `memset@GLIBC_2.2.5`, `printf@GLIBC_2.2.5`.

Rust imports: glibc (`memset`, `memcpy`, `memmove`, `bcmp`, `strlen`, `malloc`,
`calloc`, `realloc`, `free`, `posix_memalign`, `abort`, `getenv`, `getcwd`,
`readlink`, `realpath`, `open64`, `read`, `write`, `writev`, `close`, `lseek64`,
`fstat64`, `stat64`, `statx`, `mmap64`, `munmap`, `syscall`, `dl_iterate_phdr`,
`__errno_location`, `__tls_get_addr`, `__cxa_thread_atexit_impl`, `gettid`,
`pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`) and libgcc's
unwinder (`_Unwind_*`). All are libc / platform-runtime symbols provided by the
standard Rust `std` build.

`atoi` and `printf` are *implemented inside* the Rust object rather than imported
(`container_of::atoi` reproduces glibc's `(int) strtol(s, NULL, 10)` and
`container_of::print_int_line` reproduces `printf("%d\n", …)`); the differential
tests in `tests/` are what prove the reimplementations are byte-identical.

Verification command used for the check-off below:

```sh
comm -23 <(nm -D --defined-only "$C_SO"       | awk '{print $NF}' | sort) \
         <(nm -D --defined-only libdriver.so  | awk '{print $NF}' | sort)
# -> empty
```

- [x] `nm -D` shows 0 missing exported symbols in the Rust `.so`
- [x] `nm -D` shows 0 undefined non-libc/non-runtime symbols in the Rust `.so`
- [x] every C translation unit is translated (no skipped file/module)
