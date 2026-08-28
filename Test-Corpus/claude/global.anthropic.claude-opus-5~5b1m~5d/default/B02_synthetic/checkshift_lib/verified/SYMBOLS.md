# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C   `.so`: `c_src/build/libharvest-work-NSwXxz.so` (name comes from the parent
  directory name via `cmake_path(GET parent FILENAME project_name)`).
- Rust `.so`: `translation/target/release/libcheckshift_lib.so`
  (`[lib] name = "checkshift_lib"`, `crate-type = ["cdylib"]`).

Reproduce with:

```sh
nm -D --defined-only c_src/build/libharvest-work-NSwXxz.so | awk '{print $3}' | sort -u > c_syms.txt
nm -D --defined-only translation/target/release/libcheckshift_lib.so | awk '{print $3}' | sort -u > rust_syms.txt
comm -23 c_syms.txt rust_syms.txt   # missing from Rust -> MUST be empty
comm -13 c_syms.txt rust_syms.txt   # extra in Rust
```

## Symbol table

Every symbol exported by the C `.so` is also exported by the Rust `.so`, with the
exact same name. The C source has exactly one translation unit (`src/lib.c`), and
every non-`static` function in it is translated, so there is no missing module.

| # | symbol | C `.so` | Rust `.so` | C signature | Rust implementation |
|---|--------|---------|-----------|-------------|---------------------|
| 1 | `multiply_with_static` | T | T | `int (int, int)` | `pub unsafe extern "C" fn multiply_with_static` |
| 2 | `add_with_static`      | T | T | `int (int, int)` | `pub unsafe extern "C" fn add_with_static` |
| 3 | `xor_operation`        | T | T | `int (int, int)` | `pub unsafe extern "C" fn xor_operation` |
| 4 | `shift_with_static`    | T | T | `int (int, int)` | `pub unsafe extern "C" fn shift_with_static` |
| 5 | `get_operation`        | T | T | `operation_func (int)` | `pub unsafe extern "C" fn get_operation` |
| 6 | `execute_operation`    | T | T | `int (operation_func, int, int, const char*)` | `pub unsafe extern "C" fn execute_operation` |
| 7 | `compute_checksum`     | T | T | `unsigned int (int*, int)` | `pub unsafe extern "C" fn compute_checksum` |
| 8 | `init_state`           | T | T | `void (ComputeState*, int)` | `pub unsafe extern "C" fn init_state` |
| 9 | `apply_operation`      | T | T | `void (ComputeState*, int, operation_func)` | `pub unsafe extern "C" fn apply_operation` |
| 10 | `checkshift`          | T | T | `int (int, int, int, int)` | `pub unsafe extern "C" fn checkshift` |

**C exported count: 10. Rust exported count: 10.**

## Symbols intentionally NOT exported (`static` in C → file-local)

These are `static` in `src/lib.c`, so they are not dynamic symbols in the C `.so`
and must not be exported by Rust either. They are modelled as private Rust
`const`s because the C code never writes to them after initialisation.

| C declaration | Rust counterpart | exported? |
|---------------|------------------|-----------|
| `static int static_multiplier = 3;` | `const STATIC_MULTIPLIER: c_int = 3` | no (correct) |
| `static int static_addend = 100;` | `const STATIC_ADDEND: c_int = 100` | no (correct) |
| `static int static_shift_amount = 2;` | `const STATIC_SHIFT_AMOUNT: c_int = 2` | no (correct) |
| `static operation_func ops[4]` (inside `get_operation`) | local `[OperationFunc; 4]` | no (correct) |

## Undefined-symbol audit (Rust `.so`)

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports: `printf`, `malloc`, `free`, `memcpy`, `memmove`, `memset`, `bcmp`,
`calloc`, `realloc`, `posix_memalign`, `strlen`, `puts`, `abort`,
`__errno_location`, `write`, `writev`, `read`, `open64`, `close`, `lseek64`,
`stat64`, `fstat64`, `mmap64`, `munmap`, `getcwd`, `getenv`, `readlink`,
`realpath`, `syscall`, `dl_iterate_phdr`, `pthread_key_*`,
`pthread_setspecific`, `__tls_get_addr`, `_Unwind_*`, plus weak
`__cxa_finalize`, `__cxa_thread_atexit_impl`, `__gmon_start__`, `gettid`,
`statx`, `_ITM_*TMCloneTable`.

**0 missing symbols. 0 undefined non-libc symbols.**

## Stricter cross-check: full dynamic symbol table

`nm -D` can hide detail, so the tables were also compared with
`readelf --dyn-syms`, filtering to non-LOCAL defined entries:

```sh
readelf --dyn-syms --wide <so> | awk 'NR>3 && $7!="UND" && $5!="LOCAL" {print $5, $4, $8}' | sort
```

Both objects produce **exactly** the same 10 lines — all `GLOBAL FUNC`, same
names, and neither exports any extra data symbols (no `_edata` / `_end` /
`__bss_start` on either side):

```
GLOBAL FUNC add_with_static        GLOBAL FUNC init_state
GLOBAL FUNC apply_operation        GLOBAL FUNC multiply_with_static
GLOBAL FUNC checkshift             GLOBAL FUNC shift_with_static
GLOBAL FUNC compute_checksum       GLOBAL FUNC xor_operation
GLOBAL FUNC execute_operation
GLOBAL FUNC get_operation
```

The symbol diff is empty in BOTH directions, so there is no partially-translated
module: `src/lib.c` is the only C translation unit and all 10 of its non-`static`
functions are really implemented in Rust.

## Result

- [x] `nm -D` shows 0 symbols missing from the Rust `.so`.
- [x] `nm -D` shows 0 extra symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.
- [x] `readelf --dyn-syms` agrees, including symbol type and binding.
- [x] No stubs / `unimplemented!()` / fake exports: every export is a real
      translation of the corresponding C function.
- [x] Holds under both the `debug` and `release` profiles (checked by
      `./verify.sh`).

Beyond name parity, `tests/phase_d_hardening.rs::h1_*` asserts that
`get_operation(k)` actually returns the address of the exported kernel symbol in
both libraries — i.e. the `#[no_mangle]` exports are the real function bodies,
not thunks that merely share a name.
