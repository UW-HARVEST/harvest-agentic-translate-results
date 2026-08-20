# SYMBOLS.md — public symbol surface (Phase A)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#  -> c_src/build/libhello.so

# Rust
cargo build --release --offline   # -> target/release/libhello.so
cargo build --offline             # -> target/debug/libhello.so
```

## Complete C translation-unit inventory

`c_src/CMakeLists.txt` compiles exactly one translation unit into `libhello.so`:

| C file | lines | functions defined |
|--------|-------|-------------------|
| `c_src/src/hello.c` | 31 (23 of them the license header) | `helloworld` |
| `c_src/include/hello.h` | 28 (public header) | declares `int helloworld();` |

There is no other `.c`/`.h` file, no `add_library`/`add_subdirectory` beyond the
one above, and no function-renaming preprocessor macro anywhere in the public
header (`hello.h` contains only the `HELLO_H_` include guard), so the
source-level name and the final linker symbol name are identical.

## C `.so` exported (globally defined) symbols

`nm -D --defined-only c_src/build/libhello.so`:

```text
0000000000001109 T helloworld
```

Total: **1** exported symbol.

The remaining `nm -D` lines of the C `.so` are not exports:

```text
                 w _ITM_deregisterTMCloneTable   (weak, toolchain-injected)
                 w _ITM_registerTMCloneTable     (weak, toolchain-injected)
                 w __cxa_finalize@GLIBC_2.2.5    (weak, libc)
                 w __gmon_start__                (weak, toolchain-injected)
                 U puts@GLIBC_2.2.5              (undefined, libc)
```

Note `U puts`: GCC rewrote `printf("Hello World!\n")` into
`puts("Hello World!")`. This is only an implementation detail of the emitted
byte stream's producer — the bytes written to `stdout` are the 13 bytes
`Hello World!\n` either way.

## Rust `.so` exported (globally defined) symbols

`nm -D --defined-only target/release/libhello.so`:

```text
0000000000011c70 T helloworld
```

Total: **1** exported symbol.

## Symbol parity diff

```sh
diff <(nm -D --defined-only c_src/build/libhello.so    | awk '{print $NF}' | sort) \
     <(nm -D --defined-only target/release/libhello.so | awk '{print $NF}' | sort)
# (empty)
```

| # | C symbol | type | Rust `.so` exports it? | Rust definition site |
|---|----------|------|------------------------|----------------------|
| 1 | `helloworld` | `T` (global text) | YES, exact name | `src/hello.rs`, `#[unsafe(no_mangle)] pub extern "C" fn helloworld() -> c_int` |

**Missing from Rust: none.** No symbol needed a new `#[no_mangle]` wrapper and
no C translation unit was left untranslated — the whole library is the single
function above.

### Undefined (`U`) symbols in the Rust `.so`

All are libc / libgcc-unwind / Rust-std runtime imports and resolve from the
platform at load time (verified: the `.so` `dlopen`s cleanly and
`dlsym("helloworld")` succeeds — see `tests/differential.rs::phase_d_*`):

`puts`, `memcpy`, `memmove`, `memset`, `bcmp`, `strlen`, `malloc`, `calloc`,
`realloc`, `free`, `posix_memalign`, `abort`, `getenv`, `getcwd`, `readlink`,
`realpath`, `open64`, `close`, `read`, `write`, `writev`, `lseek64`, `stat64`,
`fstat64`, `statx` (weak), `mmap64`, `munmap`, `dl_iterate_phdr`, `syscall`,
`gettid` (weak), `__errno_location`, `__tls_get_addr`,
`__cxa_thread_atexit_impl` (weak), `pthread_key_create`, `pthread_key_delete`,
`pthread_setspecific`, and the `_Unwind_*` family.

**0 missing / unresolvable non-libc symbols.**

The only one of these that the translated code itself calls is `puts` (via the
`printf` binding in `src/cstdio.rs`, which LLVM rewrote to `puts` exactly as GCC
did — see the disassembly below); the rest are pulled in by Rust `std` and are
never reached on this code path.

## Disassembly cross-check (identical semantics)

```text
C:                                    Rust:
helloworld:                           helloworld:
  lea  "Hello World!"(%rip),%rax        lea  "Hello World!"(%rip),%rdi
  mov  %rax,%rdi                       call *puts@GOT(%rip)
  call puts@plt                        xor  %eax,%eax
  mov  $0x0,%eax                       ret
  ret
```

Both call `puts` with the same 12-character string and both return `0` in
`%eax`.

Profile note: the **dev** (unoptimized) Rust `.so` imports `printf` rather than
`puts`, because LLVM only folds `printf("…\n")` into `puts("…")` at
`opt-level > 0` while GCC does it at any level. Observable behaviour is
identical (`puts(s)` writes `s` then `'\n'` to `stdout`), and this is proven, not
assumed: `tests/phase_d.rs` row **D5** runs the C `.so`, the dev Rust `.so` and
the release Rust `.so` side by side over randomized call counts and requires all
three byte streams and return values to be equal.

## Verification status

| gate | result |
|------|--------|
| `nm -D --defined-only` C vs Rust (release) | identical: `{helloworld}` — diff empty |
| `nm -D --defined-only` C vs Rust (dev) | identical: `{helloworld}` — diff empty |
| C symbols missing from Rust | **0** |
| Rust symbols not in C | 0 |
| unresolved/undefined non-libc symbols in Rust | **0** (`dlopen` with `RTLD_NOW` succeeds; test D3) |
| every exported symbol actually invoked and differentially compared | yes (test D4) |
| stubs / `unimplemented!()` / `todo!()` in `src/` | none (`grep -rn 'unimplemented\|todo!\|panic!' src/` → no hits) |

Reproduce with:

```sh
./run_differential_tests.sh   # builds everything, runs Phases B, C, D
./negative_control.sh         # proves the tests reject wrong translations
```
