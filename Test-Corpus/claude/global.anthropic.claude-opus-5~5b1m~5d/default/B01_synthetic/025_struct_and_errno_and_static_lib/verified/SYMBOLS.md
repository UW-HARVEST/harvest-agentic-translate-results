# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
C   : c_src/build/libdriver.so
Rust: translation/target/release/libdriver.so
```

## Translation units in `c_src`

| C source file | translated to | status |
|---------------|---------------|--------|
| `c_src/src/driver.c` | `translation/src/lib.rs` | complete — the only `.c` file listed in `CMakeLists.txt` (`add_library(driver SHARED src/driver.c)`) |

There is exactly one C translation unit, so no module can have been skipped.

## Defined (exported) dynamic symbols

`nm -D --defined-only`:

| # | symbol | C `.so` | Rust `.so` | C declaration | notes |
|---|--------|---------|-----------|---------------|-------|
| 1 | `driver` | `T` | `T` | `void driver(const char *in);` | declared in the public header `include/driver.h` |
| 2 | `run`    | `T` | `T` | `void run(int extra_bedrooms);` | **not** in the header, but non-`static` in `driver.c`, therefore exported with external linkage. This is the lowest-level public entry point and must be tested directly. |

### Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $NF}' | sort -u) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
<empty>
```

**0 symbols missing from the Rust `.so`.**

## `static` (internal, non-exported) C functions — must NOT be exported

These have internal linkage in C and correctly have no `#[no_mangle]` wrapper in
Rust. They are covered indirectly through `driver` / `run`.

| C symbol | linkage | Rust counterpart |
|----------|---------|------------------|
| `add_floor(house_t*)` | `static` | `add_floor` (private `unsafe fn`) |
| `add_bedrooms(house_t*, int)` | `static` | `add_bedrooms` (private `unsafe fn`) |
| `add_floor_to_the_house()` | `static` | `add_floor_to_the_house` (private `unsafe fn`) |
| `print_the_house()` | `static` | `print_the_house` (private `unsafe fn`) |
| `parse_val(const char*, int*)` | `static` | `parse_val` (private `unsafe fn`) |
| `the_house` | `static` mutable object | `THE_HOUSE` (`static mut`) — process-global, **persistent across calls** |

Verified: neither `.so` exports `the_house`, `parse_val`, `add_floor`,
`add_bedrooms`, `add_floor_to_the_house` or `print_the_house`.

## Undefined (imported) dynamic symbols

`nm -D --undefined-only` — every entry must be libc / libgcc-unwind, i.e. no
dangling non-libc references.

C `.so` imports: `__errno_location`, `printf`, `puts`, `strtol` (+ weak
`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

Note: GCC rewrote `printf("An error occurred\n")` into `puts("An error
occurred")`, which is why the C `.so` imports `puts`. Both emit the identical
byte sequence `"An error occurred\n"` on the *same* `stdout` `FILE` buffer, so
this is not an observable difference.

Rust `.so` imports the same four (`__errno_location`, `printf`, `puts`,
`strtol`) plus the Rust standard-library / panic-runtime set: `_Unwind_*`,
`__tls_get_addr`, `abort`, `bcmp`, `calloc`/`malloc`/`realloc`/`free`/
`posix_memalign`, `memcpy`/`memmove`/`memset`, `strlen`, `dl_iterate_phdr`,
`open64`/`read`/`write`/`writev`/`close`/`lseek64`/`fstat64`/`stat64`/`statx`,
`mmap64`/`munmap`, `getcwd`/`getenv`/`readlink`/`realpath`, `pthread_key_*`,
`pthread_setspecific`, `syscall`, `gettid`, `__cxa_thread_atexit_impl`.

**0 undefined non-libc symbols.** (Checked with
`nm -D --undefined-only | grep -v 'GLIBC\|GCC_\|_ITM_\|__gmon_start__'`
→ empty.)

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, therefore the complete set of feature combinations is:

| # | combination | command |
|---|-------------|---------|
| 1 | default (empty) | `cargo test` |
| 2 | `--no-default-features` (identical to #1 — there is no `default` feature) | `cargo test --no-default-features` |

`verify.sh` enumerates the `[features]` table out of `Cargo.toml`
programmatically (so it will pick up any feature added later, and test every
non-empty subset), and runs symbol parity plus the full Phase B/C suite for
each combination in **both** the `debug` and `release` profiles — the release
`.so` is differentially tested too, via the `RUST_DRIVER_SO` override.

Result of `./verify.sh`:

```
=== Phase D — symbol parity ===
  [ok]   default/debug: symbol diff empty (2 C symbols all present)
  [ok]   default/debug: 0 undefined non-libc symbols
  [ok]   default/debug: no C-static symbols leaked
  [ok]   default/release: ... (same three)
  [ok]   no-default-features/debug: ... (same three)
  [ok]   no-default-features/release: ... (same three)

=== Phases B & C — differential tests ===
  [ok]   cargo test                        (against target/debug/libdriver.so)
  [ok]   cargo test                        (against target/release/libdriver.so)
  [ok]   cargo test --no-default-features  (against target/debug/libdriver.so)
  [ok]   cargo test --no-default-features  (against target/release/libdriver.so)

ALL CHECKS PASSED
```
