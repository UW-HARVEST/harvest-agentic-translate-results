# SYMBOLS.md — Phase A: exported-symbol surface

Mechanically derived from `nm -D` on both shared objects.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libharvest-work-jsRX3G.so
# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libarity_lib.so
```

The C shared-object file name is derived by `CMakeLists.txt` from the *name of
the project directory* (`cmake_path(GET parent FILENAME project_name)`), hence
`libharvest-work-jsRX3G.so` here. The test harness globs `c_src/build/*.so`
instead of hard-coding it.

## Defined (exported) symbols

All symbols come from the single translation unit `c_src/src/lib.c`; nothing in
that file is `static`, so every function it defines is exported.

| # | C symbol (`nm -D`, type) | C signature (definition) | Exported by Rust `.so` | Rust item |
|---|--------------------------|--------------------------|------------------------|-----------|
| 1 | `T shift_array`        | `void shift_array(int *arr, int size, int positions)` | ✅ `T shift_array` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn shift_array` |
| 2 | `T process_string`     | `int process_string(const char *str)` | ✅ `T process_string` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn process_string` |
| 3 | `T apply_bitmask`      | `int apply_bitmask(int value, int operation)` | ✅ `T apply_bitmask` | `#[unsafe(no_mangle)] pub extern "C" fn apply_bitmask` |
| 4 | `T init_matrix`        | `void init_matrix(int matrix[3][4])` | ✅ `T init_matrix` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn init_matrix` (`*mut c_int`, the ABI type of `int (*)[4]`) |
| 5 | `T compare_allocations`| `int compare_allocations(int val1, int val2)` | ✅ `T compare_allocations` | `#[unsafe(no_mangle)] pub extern "C" fn compare_allocations` |
| 6 | `T arity4`             | `int arity4(int, int, int, int)` | ✅ `T arity4` | `#[unsafe(no_mangle)] pub extern "C" fn arity4` |
| 7 | `T arity2`             | `int arity2(int p1, int p2)` | ✅ `T arity2` | `#[unsafe(no_mangle)] pub extern "C" fn arity2` |
| 8 | `T arity3`             | `int arity3(int p1, int p2, int p3)` | ✅ `T arity3` | `#[unsafe(no_mangle)] pub extern "C" fn arity3` |
| 9 | `T arity`              | `int arity(unsigned char len, int *params)` (definition) / `int arity(int len, int *params)` (header `include/lib.h`) | ✅ `T arity` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn arity(len: c_int, params: *const c_int)` |

**Missing from the Rust `.so`: none.** No module of the C source was left
untranslated: `c_src` contains exactly one implementation file (`src/lib.c`,
181 lines) and one header (`include/lib.h`), and all nine functions of that file
have real (non-stub) Rust implementations.

There is no `DataBlock` symbol to match: it is a `typedef`ed struct used only as
a local variable inside `arity4`, so it contributes no linker symbol. It is
still mirrored in Rust as `#[repr(C)] struct DataBlock` for layout fidelity.

## Symbol-name diff (Phase D gate)

```sh
diff <(nm -D --defined-only c_src/build/*.so            | awk '{print $NF}' | sort) \
     <(nm -D --defined-only translation/target/release/libarity_lib.so | awk '{print $NF}' | sort)
```

Result: **empty diff** — the two libraries export exactly the same nine names
(verified for both the `debug` and the `release` Rust profile).

## Undefined (imported) symbols

| library | undefined symbols |
|---------|-------------------|
| C | `malloc`, `free`, `memmove`, `strlen` (all `@GLIBC_2.2.5`), plus the weak toolchain symbols `_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`, `__cxa_finalize`, `__gmon_start__` |
| Rust | the same four libc functions plus libc/`libgcc` runtime imports pulled in by `std` (`memcpy`, `memset`, `calloc`, `realloc`, `posix_memalign`, `abort`, `__errno_location`, `pthread_key_*`, `dl_iterate_phdr`, `open64`/`read`/`write`/`close`/`stat64`, `mmap64`/`munmap`, `_Unwind_*` from `libgcc`, …) |

**0 missing / unresolvable non-libc symbols in the Rust `.so`**: every undefined
symbol is provided by glibc or `libgcc_s` (the `_Unwind_*` family), both of which
are already dependencies of any C program. Verified with:

```sh
ldd -r translation/target/release/libarity_lib.so   # no "undefined symbol" lines
```

## Feature / configuration matrix

`translation/Cargo.toml` declares **no `[features]` table**, so the only
feature combination that exists is the default (empty) one; `--no-default-features`
and `--all-features` select the very same code. The build dimensions that *do*
exist are the cargo profiles, and both are covered by the suites:

| configuration | Rust `.so` under test | how |
|---|---|---|
| default features, `dev` profile (panic = unwind, debug-assertions on) | `target/debug/libarity_lib.so` | `cargo test` |
| default features, `release` profile (`panic = "abort"`, opt-level 3) | `target/release/libarity_lib.so` | `cargo test --release` |
| `--no-default-features` / `--all-features` (aliases of default) | both profiles | `./run_all.sh` |
| each profile's tests against the *other* profile's `.so` | crosswise | `RUST_LIB_PATH=... cargo test` (driven by `run_all.sh`) |

Both profiles matter and are not interchangeable: `dev` enables the Rust
UB checks that made two of the divergences in `FINDINGS.md` observable, while
`release` enables the LLVM optimisation that made the third observable.

`./run_all.sh` verifies the symbol diff and runs both differential suites for
every one of these configurations; the last recorded run reported
`ALL CONFIGURATIONS PASSED` with `symbols: identical (9 exported)` and
`ldd -r: no undefined symbols` in each.
