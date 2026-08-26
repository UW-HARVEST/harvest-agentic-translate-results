# SYMBOLS.md — Symbol parity: C `.so` vs Rust `.so`

Derived mechanically from `nm -D` on both shared objects.

```
C   : c_src/build/libdriver.so   (cmake, default config)
Rust: target/debug/libdriver.so  (cargo, crate-type = ["cdylib"])
```

## Build configurations

`Cargo.toml` has **no `[features]` section** and `c_src/CMakeLists.txt` has **no
`option()` / `add_definitions()` / conditional branches**. The only `#if` in the
C source is the `DRIVER_H_` include guard in `driver.h`. Therefore there is
exactly **ONE** build configuration:

| # | configuration | cargo invocation |
|---|---------------|------------------|
| 1 | default (empty feature set) | `cargo check --no-default-features` == `cargo check` |

There is no feature cross-product to enumerate; Phase D's "repeat for every
feature combination" collapses to this single combination.

## Exported (defined) symbols

`nm -D --defined-only`:

| # | symbol | C | Rust | signature | status |
|---|--------|---|------|-----------|--------|
| 1 | `driver` | `T` | `T` | `void driver(const char *in)` | ✅ present in both |
| 2 | `run`    | `T` | `T` | `void run(int extra_bedrooms)`  | ✅ present in both |

**Symbol diff (C exported − Rust exported) = ∅.** Both symbols are exported by
the Rust `.so` under the exact same names via `#[unsafe(no_mangle)] pub extern "C"`.

### `static` C functions — correctly NOT exported

These have internal linkage in C and appear in neither `.so`'s dynamic table.
They are translated as private Rust `fn`s, which is the correct parity:

| C symbol | linkage | Rust counterpart | exported? |
|----------|---------|------------------|-----------|
| `add_floor`             | `static` | `fn add_floor`             | no (correct) |
| `add_bedrooms`          | `static` | `fn add_bedrooms`          | no (correct) |
| `add_floor_to_the_house`| `static` | `fn add_floor_to_the_house`| no (correct) |
| `print_the_house`       | `static` | `fn print_the_house`       | no (correct) |
| `parse_val`             | `static` | `fn parse_val`             | no (correct) |
| `the_house`             | `static` object | `static THE_HOUSE: TheHouse` | no (correct) |

No whole C module was skipped: `c_src/src/` contains exactly one translation
unit (`driver.c`), fully translated in `src/lib.rs`.

## Undefined / imported symbols

`nm -D --undefined-only`. Requirement: **0 missing/undefined non-libc symbols**
in the Rust `.so`.

C imports: `__errno_location`, `printf`, `puts`, `strtol` (+ weak
`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

Rust imports the same libc trio it needs (`__errno_location`, `printf`,
`strtol`) plus the standard Rust-runtime set, all of which resolve from
`libc.so.6` / `libgcc_s.so.1` / `ld-linux-x86-64.so.2` (see `objdump -p`
`NEEDED`):

* libc: `abort bcmp calloc close dl_iterate_phdr free fstat64 getcwd getenv
  lseek64 malloc memcpy memmove memset mmap64 munmap open64 posix_memalign
  printf pthread_key_* read readlink realloc realpath stat64 strlen strtol
  syscall write writev __errno_location __tls_get_addr` (+ weak `gettid`,
  `statx`, `__cxa_thread_atexit_impl`, `__cxa_finalize`)
* libgcc unwinder: `_Unwind_*`

**Non-libc undefined symbols in the Rust `.so`: 0.** ✅

## Note: `printf` vs `puts` in the C `.so`

The C `.so` imports `puts` because gcc rewrites the constant-string call
`printf("An error occurred\n")` into `puts("An error occurred")` (it strips the
trailing newline; `puts` re-appends it). This is a code-generation detail only —
the bytes written to `stdout` are identical to the Rust translation's
`printf("An error occurred\n")`. Confirmed byte-for-byte by the Phase C tests.
