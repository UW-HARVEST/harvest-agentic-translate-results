# SYMBOLS.md — Symbol-surface parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects. No assumptions.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libhello.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libhello.so
```

## Translation-unit inventory (completeness check)

Every C source file must have a Rust counterpart. A whole untranslated
`.c` file is the main way symbols go missing, so the file list is checked
first, not just the symbol list.

| C source file | translated in | status |
|---|---|---|
| `c_src/src/hello.c` | `translation/src/lib.rs` | translated |

`find c_src -name '*.c'` returns exactly one file, so no module was skipped.
`find c_src -name '*.h'` returns exactly one header (`include/hello.h`),
which declares exactly one function. There are no macro-generated /
namespace-prefixed symbol names in the header (no function-like macros at
all — the only preprocessor directive is the `HELLO_H_` include guard), so
the exported name set is not expanded by macro trickery.

## Exported (defined, dynamic) symbols

| # | symbol | C `.so` | Rust `.so` | C binding | Rust binding | status |
|---|--------|---------|------------|-----------|--------------|--------|
| 1 | `helloworld` | yes (`T`) | yes (`T`) | global text | global text | MATCH |

Missing from Rust `.so`: **none**.
Extra in Rust `.so` (not in C): **none**.

Rust source of the export: `#[unsafe(no_mangle)] pub extern "C" fn helloworld() -> c_int`
in `translation/src/lib.rs`. `crate-type = ["cdylib"]` in `Cargo.toml`, so the
symbol is exported from a real shared object and is reachable by `dlsym`,
which is how every test in `tests/` calls it.

## Undefined (imported) symbols

Only libc / unwinder imports are permitted. None of the imports is an
untranslated project symbol.

| `.so` | undefined symbols | all libc/libgcc? |
|---|---|---|
| C | `puts`, `__cxa_finalize`, `__gmon_start__`, `_ITM_*register TMCloneTable` | yes |
| Rust | `puts`, `memcpy`, `memmove`, `memset`, `bcmp`, `strlen`, `malloc`, `calloc`, `realloc`, `free`, `posix_memalign`, `abort`, `getenv`, `getcwd`, `readlink`, `realpath`, `open64`, `close`, `read`, `write`, `writev`, `lseek64`, `stat64`, `fstat64`, `statx`, `mmap64`, `munmap`, `syscall`, `gettid`, `dl_iterate_phdr`, `__errno_location`, `__tls_get_addr`, `pthread_key_{create,delete}`, `pthread_setspecific`, `__cxa_thread_atexit_impl`, `__cxa_finalize`, `__gmon_start__`, `_Unwind_*`, `_ITM_*` | yes |

The extra Rust imports are the Rust standard library's own runtime
(allocator, panic/unwind machinery, `std::fs`/`std::env` support pulled in
by libstd), not project code. `ldd` on the Rust `.so` resolves to only
`libgcc_s.so.1`, `libc.so.6` and the loader — no unresolved project
dependency.

Note that **both** objects import `puts`, not `printf`: the C compiler and
LLVM each apply the standard `printf("...\n")` → `puts("...")` strength
reduction. `puts` appends the newline itself, so the emitted bytes are
identical, and both write through the *same* libc `stdout` `FILE` buffer.

## Gate

- [x] `nm -D` shows **0** symbols missing from the Rust `.so`.
- [x] `nm -D` shows **0** undefined non-libc symbols in the Rust `.so`.
- [x] No stubs / `unimplemented!()` / `todo!()` anywhere — the single symbol
      is a genuine translation of the C body
      (`grep -rn 'unimplemented!\|todo!\|panic!' translation/src/` is empty).
