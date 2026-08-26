# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust (only one configuration exists; see CONFIGS.md "Feature combinations")
cargo build --no-default-features            # -> target/debug/libdriver.so
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C file | translated to | status |
|--------|---------------|--------|
| `c_src/src/driver.c` (99 lines) | `src/lib.rs` | fully translated |
| `c_src/include/driver.h` (29 lines) | declarations mirrored in `src/lib.rs` | fully translated |

No C source file is missing from the Rust translation, so no Phase A
"translate the missing module" work is required.

## Exported (defined) dynamic symbols

`nm -D --defined-only <so>`, function/data symbols only:

| # | symbol | C `.so` | Rust `.so` | C declaration | notes |
|---|--------|---------|------------|---------------|-------|
| 1 | `printLine`        | T | T | `void printLine(const char *line)` | `driver.c:30` |
| 2 | `printHexCharLine` | T | T | `void printHexCharLine(char charHex)` | `driver.c:38` |
| 3 | `bad`              | T | T | `void bad(void)`  | `driver.c:43` |
| 4 | `good`             | T | T | `void good(void)` | `driver.c:84` |
| 5 | `driver`           | T | T | `void driver(int useGood)` | `driver.c:90`, the only symbol in `driver.h` |

### Deliberately NOT exported (`static` in C, private in Rust)

| C symbol | linkage | Rust counterpart |
|----------|---------|------------------|
| `goodG2B` | `static void goodG2B()` — internal | `unsafe fn goodG2B()` (private, not `#[no_mangle]`) |
| `goodB2G` | `static void goodB2G()` — internal | `unsafe fn goodB2G()` (private, not `#[no_mangle]`) |

These are correctly absent from both `.so` files. Adding `#[no_mangle]`
wrappers for them would be a *divergence*, not a fix.

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/debug/libdriver.so \
           | grep -E ' (T|B|D|W|R) ' | awk '{print $NF}' | sort)
(empty)
```

**Result: 0 symbols missing from the Rust `.so`.** (Reproduced by
`tests/phase_d_symbols.rs::c_exported_symbols_are_all_exported_by_rust`, which
shells out to `nm -D` and asserts the diff is empty, and by
`harness::Api::load`, which resolves all 5 symbols in both libraries via
`libloading` during every test — a missing symbol would fail to resolve.)

## Undefined symbols in the Rust `.so`

`nm -D -u target/debug/libdriver.so` lists only libc / libgcc-unwind imports:

`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `_Unwind_*@GCC_*`,
`__cxa_finalize`, `__cxa_thread_atexit_impl`, `__errno_location`,
`__gmon_start__`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`,
`dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`,
`malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`,
`posix_memalign`, **`printf@GLIBC_2.2.5`**, `pthread_key_create`,
`pthread_key_delete`, `pthread_setspecific`, `read`, `readlink`, `realloc`,
`realpath`, `stat64`, `statx`, `strlen`, `syscall`, `write`, `writev`.

**0 missing/undefined non-libc symbols.** The only *intentional* import that
matters for behaviour is `printf@GLIBC_2.2.5`: the Rust translation calls the
same variadic libc `printf` as the C code, through the same process-global
`stdout` `FILE`, so formatting, integer promotion and stdio buffering are
byte-identical by construction rather than re-implemented.

## ABI check on the narrow (`char`) parameter

`printHexCharLine` takes a `char`. Under the SysV AMD64 psABI the caller may
leave the upper 24 bits of `%edi` undefined, so the callee must narrow. gcc does
so at every optimisation level; the Rust originally did **not** once optimised.
That was a real divergence and it was fixed — see the "Divergence found and
fixed (row E9)" section of `ERRORS.md`. Final state:

| build | narrowing instruction | narrows? |
|-------|----------------------|----------|
| C, `cmake` default (`-O0`) | `mov %edi,%eax; mov %al,-0x4(%rbp); movsbl -0x4(%rbp),%eax` | yes |
| C, `gcc -O2` | `movsbl %dil,%esi` | yes |
| Rust `dev` (after fix) | `movsbl` on the low byte | yes |
| Rust `release` (after fix) | `movsbl %dil,%esi` | yes — instruction-identical to `gcc -O2` |

Asserted behaviourally by
`tests/phase_c_errors.rs::e9_print_hex_char_line_out_of_range_int_across_ffi`
and `generic_one_step_past_every_documented_range`, which call the symbol
through a deliberately widened `extern "C" fn(c_int)` prototype, and by
`tests/phase_d_symbols.rs::optimised_c_build_agrees_with_rust_on_the_whole_surface`,
which compares against a `-O2` rebuild of the C.

## Codegen parity note: `printf` → `puts`

Optimised builds rewrite `printf("%s\n", s)` into `puts(s)`. `gcc -O2` performs
the *identical* transform on `printLine` (`test %rdi,%rdi; je; jmp puts@plt`),
so the `puts` import that appears in the release Rust `.so`'s undefined-symbol
list is codegen parity with the optimised C, not a behavioural change. Confirmed
byte-identical for NULL, empty, `%`-bearing, non-UTF-8 and 70 KB payloads.
