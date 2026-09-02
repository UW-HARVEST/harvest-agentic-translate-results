# SYMBOLS.md — Phase A symbol map

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

* C object:    `c_src/build/libdriver.so`
* Rust object: `translation/target/release/libdriver.so`

## C source inventory (completeness check)

The whole C subtree is two files; `CMakeLists.txt` compiles exactly one of them
into the `driver` shared library:

| C file | role | translated? |
|--------|------|-------------|
| `c_src/include/driver.h` | public header, declares `void driver(int x, int y);` behind the `DRIVER_H_` guard | yes (declaration only, no code) |
| `c_src/src/driver.c` | the only source file in `add_library(driver SHARED src/driver.c)` | yes — `translation/src/lib.rs::driver` |

No C module/file is missing from the translation, so no Phase A "translate the
skipped module" work is required.

The C is written with ISO digraphs plus `<iso646.h>` alternative operator
spellings (`%:` = `#`, `<%` = `{`, `%>` = `}`, `bitor` = `|`, `compl` = `~`),
so `driver` is:

```c
void driver(int x, int y) {
    int result = x | ~y;
    printf("%d", result);
    puts("");
}
```

There are no namespace/prefix/renaming macros anywhere in the C, therefore no
macro-generated linker symbols to mirror.

## Exported (defined) dynamic symbols

`nm -D --defined-only`:

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `driver` | `T driver` | `T driver` | PRESENT in both |

Symbol diff (C-exported minus Rust-exported): **empty**.

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort -u) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
(no output)
```

Nothing needed a new `#[no_mangle]` wrapper and nothing needed translating; the
Rust side already exports `driver` via `#[unsafe(no_mangle)] pub extern "C"`.

## Undefined / imported symbols

| object | undefined non-weak symbols |
|--------|----------------------------|
| C | `printf@GLIBC_2.2.5`, `puts@GLIBC_2.2.5` |
| Rust | `printf@GLIBC_2.2.5`, `putchar@GLIBC_2.2.5`, plus the Rust std runtime's libc/unwind set (`malloc`, `free`, `calloc`, `realloc`, `posix_memalign`, `memcpy`, `memmove`, `memset`, `bcmp`, `strlen`, `abort`, `__errno_location`, `write`, `writev`, `read`, `open64`, `close`, `lseek64`, `stat64`, `fstat64`, `readlink`, `realpath`, `getcwd`, `getenv`, `mmap64`, `munmap`, `dl_iterate_phdr`, `syscall`, `pthread_key_create`/`_delete`/`setspecific`, `__tls_get_addr`, `_Unwind_*`) |

**0 missing, 0 undefined non-libc symbols on the Rust side.** Every Rust
undefined symbol resolves out of `libc`/`libgcc_s` (the C runtime and the
unwinder), which is exactly what the C `.so` also relies on.

Note on `puts` vs `putchar`: the Rust build is compiled with optimisation, and
LLVM rewrites the `puts("")` call into the equivalent `putchar('\n')`. Both go
through the same `stdout` `FILE` buffer and emit the identical single `\n` byte,
so the observable byte stream is unchanged. The differential tests in
`tests/differential.rs` capture real `stdout` bytes and confirm this.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section**, so the crate has
exactly one configuration: the default (empty) feature set. There is likewise no
`#if`/`%:if` conditional compilation in the C beyond the header include guard.
The complete feature matrix to verify is therefore:

| # | cargo invocation | notes |
|---|------------------|-------|
| 1 | `cargo test` | default = only possible configuration |
| 2 | `cargo test --no-default-features` | identical to #1 (no features exist) |

## Completion gate (verified)

| gate | result |
|------|--------|
| `SYMBOLS.md`: `nm -D` shows 0 missing / 0 undefined non-libc symbols in Rust | PASS — `comm -23` of the C's exported set against the Rust's is empty for `target/release/libdriver.so`, `target/release/deps/libdriver.so` and `target/debug/deps/libdriver.so`; the only Rust-side undefined symbols are libc/libgcc |
| Phase B: every one of the 23 `CONFIGS.md` rows passes across randomized inputs | PASS — 23/23 checked |
| Phase C: every one of the 10 `ERRORS.md` rows has a passing differential test | PASS — 10/10 checked |
| Holds under every feature combination | PASS — `./run_all_combos.sh` enumerates the feature powerset from `Cargo.toml` (empty) and runs `<default>`, `--no-default-features`, `--all-features` against both the debug and release profiles: 6 configurations x 35 cases, all green |

Test target: `tests/differential.rs`, run with `cargo test` / `cargo test --release`.
It uses `harness = false` because libtest writes its own progress to file
descriptor 1 — the very channel `driver` writes to — which would otherwise be
interleaved into the captured byte stream. Progress is reported on stderr
instead, so a capture contains nothing but the bytes the library emitted.

### Non-vacuity check

The suite was confirmed to actually detect divergence by mutating the Rust and
re-running (both mutations reverted afterwards):

| mutation to `src/lib.rs` | detected |
|--------------------------|----------|
| `x \| !y` → `x & !y` | 32 of 35 cases failed |
| `puts("")` newline removed | 32 of 35 cases failed |

(The cases that still pass under a mutation are the ones whose inputs are
insensitive to it — e.g. the zero-call row, and pairs where `x & !y == x \| !y`.)
