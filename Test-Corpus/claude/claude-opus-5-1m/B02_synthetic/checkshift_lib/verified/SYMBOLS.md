# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
C   : c_src/build/libtranslated_rust.so   (cmake, default build type -> -O0)
Rust: target/debug/libcheckshift_lib.so   (cargo build, crate-type = ["cdylib"])
```

Regenerate / re-verify with:

```sh
./symbol_parity.sh
```

## Build-time configuration surface

| source | configurations |
|---|---|
| `Cargo.toml` `[features]` | **absent** — the only valid combination is the empty feature set (`--no-default-features`) |
| `c_src/CMakeLists.txt` | no `option()`, no `add_definitions`, no `#ifdef` in `lib.c`; single `SHARED` target from `src/lib.c` |

=> exactly **one** build configuration to verify. Enumerated mechanically by
`./check_all_features.sh` (which prints `features found: 0`).

## Defined (exported) symbols

`nm -D --defined-only` on the C `.so`, and the matching entry in the Rust `.so`:

| # | C symbol | C decl | in Rust `.so`? | Rust item |
|---|----------|--------|----------------|-----------|
| 1 | `multiply_with_static` | `int multiply_with_static(int, int)` | ✅ `T` | `src/ops.rs::multiply_with_static` |
| 2 | `add_with_static` | `int add_with_static(int, int)` | ✅ `T` | `src/ops.rs::add_with_static` |
| 3 | `xor_operation` | `int xor_operation(int, int)` | ✅ `T` | `src/ops.rs::xor_operation` |
| 4 | `shift_with_static` | `int shift_with_static(int, int)` | ✅ `T` | `src/ops.rs::shift_with_static` |
| 5 | `get_operation` | `operation_func get_operation(int)` | ✅ `T` | `src/ops.rs::get_operation` |
| 6 | `execute_operation` | `int execute_operation(operation_func, int, int, const char*)` | ✅ `T` | `src/ops.rs::execute_operation` |
| 7 | `compute_checksum` | `unsigned int compute_checksum(int*, int)` | ✅ `T` | `src/state.rs::compute_checksum` |
| 8 | `init_state` | `void init_state(ComputeState*, int)` | ✅ `T` | `src/state.rs::init_state` |
| 9 | `apply_operation` | `void apply_operation(ComputeState*, int, operation_func)` | ✅ `T` | `src/state.rs::apply_operation` |
| 10 | `checkshift` | `int checkshift(int, int, int, int)` (the only `include/lib.h` decl) | ✅ `T` | `src/checkshift.rs::checkshift` |

**Symbols in C but missing from Rust: 0.**
**No macro-generated exports exist** (`STRINGIFY` / `LOG_VALUE` expand only
inside function bodies, producing no symbols).

C-internal, deliberately *not* exported (they are `static` in `lib.c`, so they
are absent from the C `.so`'s dynamic table too — parity requires they stay
private in Rust as well):

| C `static` | Rust counterpart | exported by either `.so`? |
|---|---|---|
| `static int static_multiplier = 3` | `src/ops.rs::STATIC_MULTIPLIER` | no / no |
| `static int static_addend = 100` | `src/ops.rs::STATIC_ADDEND` | no / no |
| `static int static_shift_amount = 2` | `src/ops.rs::STATIC_SHIFT_AMOUNT` | no / no |
| `static operation_func ops[4]` (fn-local, lazily filled) | `src/ops.rs::OPS` | no / no |

`typedef struct ComputeState` / `typedef ... operation_func` are types, not
symbols; their ABI is verified behaviourally in Phase B (rows C11–C13: structs
and function pointers are handed *across* the two `.so`s).

## Undefined (imported) symbols

Requirement: 0 missing/undefined **non-libc** symbols in the Rust `.so`.

C `.so` imports: `malloc`, `free`, `memcpy`, `printf`, `puts`
(`puts` is gcc's `-O0`/`-O2` rewrite of `printf` calls that carry no conversion
specifier — same bytes on stdout), plus the usual weak
`_ITM_*` / `__cxa_finalize` / `__gmon_start__`.

Rust `.so` imports: the same `malloc`/`free`/`memcpy`/`printf` plus
glibc/`libgcc` runtime entries only —
`memmove memset bcmp strlen calloc realloc posix_memalign abort __errno_location
__tls_get_addr pthread_key_* open64 close read write writev lseek64 fstat64
stat64 statx mmap64 munmap getcwd getenv readlink realpath syscall gettid
dl_iterate_phdr _Unwind_*` — i.e. **libc + libgcc_s unwinder only, 0 non-libc
undefined symbols.** Verified by `./symbol_parity.sh`, which fails if any
undefined symbol falls outside that allow-list.
