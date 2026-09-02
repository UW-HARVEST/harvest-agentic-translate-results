# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C   `.so`: `c_src/build/libharvest-work-tJDXtz.so` (built via CMake, `src/lib.c`)
* Rust `.so`: `translation/target/release/libbuffapp_lib.so` (`crate-type = ["cdylib"]`)

Reproduce with `translation/tests/symbol_parity.sh` or:

```sh
nm -D --defined-only c_src/build/libharvest-work-tJDXtz.so | awk '$2=="T"{print $3}' | sort
nm -D --defined-only translation/target/release/libbuffapp_lib.so | awk '$2=="T"{print $3}' | sort
```

## Public (defined, `T`) symbols

`lib.c` has no `static` functions, so every function it defines is exported.
The header (`include/lib.h`) only declares `buffapp`, but the ABI surface of the
`.so` is all six functions; all six are treated as public API here.

| # | symbol | C `.so` | Rust `.so` | declared in `lib.h` | notes |
|---|--------|---------|------------|---------------------|-------|
| 1 | `create_buffer`     | T | T | no  | returns `StringBuffer*` (malloc'd), NULL on alloc failure |
| 2 | `append_to_buffer`  | T | T | no  | returns `int` (0 ok, -1 realloc failure) |
| 3 | `destroy_buffer`    | T | T | no  | `void`, NULL-tolerant |
| 4 | `get_operation_name`| T | T | no  | returns `const char*` into static storage |
| 5 | `perform_operation` | T | T | no  | returns `int` |
| 6 | `buffapp`           | T | T | yes | returns `int`, also writes to stdout via `printf` |

**Missing from Rust `.so`: none (0).** No `#[no_mangle]` wrapper had to be
added and no C module was untranslated — `c_src` contains exactly one
translation unit (`src/lib.c`) and every function in it has a Rust counterpart
in `translation/src/lib.rs`.

## Non-public data symbols

Neither `.so` exports any object (`D`/`B`/`R`) symbols of its own. The Rust
`.so` additionally exports Rust-mangled `_ZN…` std/core symbols and the usual
`cdylib` boilerplate (`_init`, `_fini`, `__rust_*` allocator shims); these are
implementation artifacts of the Rust runtime, not part of the C ABI surface, and
are not required for parity in the C → Rust direction.

## Undefined (imported) symbols

The Rust `.so` must not import anything outside libc / the platform unwinder.

C `.so` imports: `malloc`, `realloc`, `free`, `strlen`, `strcpy`, `strcmp`,
`sprintf`, `printf` (all `GLIBC_2.2.5`), plus weak
`_ITM_{de,}registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__`.

Rust `.so` imports: the same eight libc functions, plus additional libc
(`memcpy`, `memmove`, `memset`, `calloc`, `posix_memalign`, `bcmp`, `abort`,
`getenv`, `getcwd`, `readlink`, `realpath`, `open64`, `read`, `write`, `writev`,
`close`, `lseek64`, `fstat64`, `stat64`, `mmap64`, `munmap`, `syscall`,
`dl_iterate_phdr`, `__errno_location`, `__tls_get_addr`, `pthread_key_*`,
`pthread_setspecific`) and `_Unwind_*` from `libgcc_s`.

**0 missing / 0 undefined non-libc symbols in the Rust `.so`.** Every extra
import is glibc or the platform unwinder, both of which are already present in
any process that loads the C `.so`.

## Phase D verification result

`translation/tests/symbol_parity.sh` (run it directly; exits non-zero on any gap):

```
C   .so: c_src/build/libharvest-work-tJDXtz.so   (6 exported T symbols)
Rust .so: translation/target/release/libbuffapp_lib.so (6 exported T symbols)

--- C symbols missing from the Rust .so ---
(none)
--- unresolved symbols in the Rust .so (ldd -r, authoritative) ---
(none)
--- dlopen + dlsym check on every C symbol via the Rust .so ---
  ok      append_to_buffer
  ok      buffapp
  ok      create_buffer
  ok      destroy_buffer
  ok      get_operation_name
  ok      perform_operation

SYMBOL PARITY: PASS (0 missing, 0 non-libc undefined)
```

In addition, the test harness resolves all six symbols with `dlsym` at load
time (`tests/support/mod.rs`), so a missing export fails every test rather than
being silently skipped.

`ldd -r` is used instead of comparing against a hand-written libc allow-list: it
performs full relocation processing and reports anything the dynamic loader
cannot resolve, which is the property that actually matters.
