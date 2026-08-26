# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared libraries.

```
C   .so : c_src/build/libtranslated_rust.so   (cmake, default config)
Rust.so : target/debug/libhatch_lib.so        (crate-type = ["cdylib"])
```

Reproduce with:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so \
  | awk '$2=="T"||$2=="D"||$2=="B"||$2=="W"{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only target/debug/libhatch_lib.so \
  | awk '$2=="T"||$2=="D"||$2=="B"||$2=="W"{print $3}' | sort > /tmp/r_syms.txt
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt   # must be empty
```

The check is also automated as an integration test: `tests/symbol_parity.rs`.

## Symbol table

C `.so` exports exactly 12 defined `T` symbols (the whole translation unit is
one file, `c_src/src/lib.c`; there is no second C module, so there is no
"untranslated file" gap).

| # | symbol | C signature (`c_src/src/lib.c`) | in C `.so` | in Rust `.so` | Rust item |
|---|--------|--------------------------------|-----------|--------------|-----------|
| 1 | `increment_counter` | `void increment_counter(int value, int unused_param)` | T | T | `pub extern "C" fn increment_counter` |
| 2 | `update_accumulator` | `void update_accumulator(int value, int unused_param)` | T | T | `pub extern "C" fn update_accumulator` |
| 3 | `apply_operation` | `int apply_operation(operation_func op, int a, int b, int c)` | T | T | `pub extern "C" fn apply_operation` |
| 4 | `add_three` | `int add_three(int a, int b, int c)` | T | T | `pub extern "C" fn add_three` |
| 5 | `multiply_add` | `int multiply_add(int a, int b, int c)` | T | T | `pub extern "C" fn multiply_add` |
| 6 | `complex_calc` | `int complex_calc(int a, int b, int c)` | T | T | `pub extern "C" fn complex_calc` |
| 7 | `shift_array_data` | `void shift_array_data(int *arr, int size, int shift_by)` | T | T | `pub unsafe extern "C" fn shift_array_data` |
| 8 | `process_pointer_data` | `int process_pointer_data(int *ptr, int multiplier)` | T | T | `pub unsafe extern "C" fn process_pointer_data` |
| 9 | `compute_with_dynamic_memory` | `int compute_with_dynamic_memory(int base, int count)` | T | T | `pub extern "C" fn compute_with_dynamic_memory` |
| 10 | `get_time_based_value` | `int get_time_based_value(int seed)` | T | T | `pub extern "C" fn get_time_based_value` |
| 11 | `manipulate_records` | `int manipulate_records(DataRecord *records, int num_records, int shift)` | T | T | `pub unsafe extern "C" fn manipulate_records` |
| 12 | `hatch` | `int hatch(int param1, int param2, int param3, int param4)` | T | T | `pub extern "C" fn hatch` |

**Missing from Rust `.so`: none (`comm -23` is empty).**

## Non-exported C internals (deliberately not symbols)

| C construct | kind | Rust counterpart | exported? |
|---|---|---|---|
| `static int global_counter = 0;` | file-scope state | `static GLOBAL_COUNTER: AtomicI32` | no (`static`, correctly not in `nm -D` for either lib) |
| `static int global_accumulator = 0;` | file-scope state | `static GLOBAL_ACCUMULATOR: AtomicI32` | no |
| `typedef int (*operation_func)(int,int,int)` | typedef | `pub type OperationFunc` | n/a (type) |
| `typedef void (*modifier_func)(int,int)` | typedef | `pub type ModifierFunc` | n/a (type) |
| `typedef struct {...} DataRecord` | typedef | `#[repr(C)] pub struct DataRecord` | n/a (type) |
| `include/lib.h` declares only `int hatch(int,int,int,int)` | header | — | the other 11 are still `extern` in the `.so` and are tested directly |

`DataRecord` ABI (x86-64 Linux) — must match byte-for-byte because
`manipulate_records` takes a caller-allocated array and strides through it:

| field | C offset | C size | Rust field | Rust offset |
|---|---|---|---|---|
| `int id` | 0 | 4 | `id: c_int` | 0 |
| `int value` | 4 | 4 | `value: c_int` | 4 |
| `time_t timestamp` | 8 | 8 | `timestamp: TimeT (i64)` | 8 |
| `char name[32]` | 16 | 32 | `name: [c_char; 32]` | 16 |
| **total** | — | **48** (align 8) | — | **48** |

This layout is verified *behaviourally* by `tests/valid_paths.rs`
(`manipulate_records` rows): the test builds the array using the Rust
`#[repr(C)]` layout and feeds the very same bytes to the **C** `.so`; any
stride/offset mismatch changes the returned sum.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only target/debug/libhatch_lib.so` lists only libc / libgcc
runtime imports (`malloc`, `free`, `memcpy`, `memmove`, `memset`, `clock_gettime`,
`pthread_*`, `_Unwind_*`, `__errno_location`, …). **0 missing non-libc symbols.**

## Build-time configuration surface

`Cargo.toml` has **no `[features]` table**, and `c_src/CMakeLists.txt` has no
`option()`, no `target_compile_definitions`, and no `#ifdef` in `lib.c`
(`grep -c '#if' c_src/src/lib.c` → 0). Therefore the complete set of valid
feature combinations is the single empty/default one:

| # | cargo invocation | notes |
|---|---|---|
| 1 | `cargo check --no-default-features` | == default; no features exist |
| 2 | `cargo check` | identical to #1 |

Both are exercised by `run_all.sh`.

## Results

```
$ cargo test --test symbol_parity -- --nocapture
C exports 12 defined symbols
Rust exports 12 defined symbols
symbol diff (C \ Rust): EMPTY — 12 symbols matched
libtranslated_rust.so: 11 undefined symbols, 0 non-runtime
libhatch_lib.so:       52 undefined symbols, 0 non-runtime
libtranslated_rust.so: RTLD_NOW ok, all 12 symbols resolvable via dlsym
libhatch_lib.so:       RTLD_NOW ok, all 12 symbols resolvable via dlsym
Rust-only exported symbols (0): []
test result: ok. 4 passed
```

* **`comm -23 c_syms rust_syms` is EMPTY** for the debug *and* the release
  `cdylib`. No C source file was skipped, so no module had to be translated to
  close the gap — `c_src/src/lib.c` is the only translation unit and all 12 of
  its external functions are present, exported, and behaviourally verified.
* The Rust `.so` exports **no extra** C-API surface (`Rust \ C` is empty apart
  from the ELF/Rust-runtime internals filtered by the test).
* Import completeness is checked mechanically rather than by allowlist: an
  undefined symbol is accepted only if it is weak or bound to a versioned
  platform runtime (`@GLIBC_*`, `@GCC_*`, `@GLIBCXX_*`, `@CXXABI_*`, `@LIBC*`).
  Both libraries have **0** unversioned strong undefined symbols.
* `phase_d_both_so_resolve_eagerly` additionally `dlopen`s each library with
  `RTLD_NOW`, forcing the dynamic linker to bind every undefined symbol
  immediately — the definitive proof that nothing is missing — and then `dlsym`s
  all 12 names.
