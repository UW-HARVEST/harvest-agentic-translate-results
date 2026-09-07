# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` on the C shared library
`c_src/build/libharvest-work-vsTrFn.so`, compared against the Rust
`translation/target/release/libfallcalc_lib.so`.

Reproduce with:

```sh
nm -D --defined-only c_src/build/*.so            | awk '{print $3}' | sort > /tmp/c_syms
nm -D --defined-only translation/target/release/libfallcalc_lib.so | awk '{print $3}' | sort > /tmp/r_syms
comm -23 /tmp/c_syms /tmp/r_syms   # must be EMPTY
```

## Exported (dynamic, defined) symbols

| # | symbol | C `.so` | Rust `.so` | C source | notes |
|---|--------|---------|------------|----------|-------|
| 1 | `safe_double_to_int` | ✅ | ✅ | `src/lib.c:47` | `int safe_double_to_int(double)` |
| 2 | `process_array_reverse` | ✅ | ✅ | `src/lib.c:65` | `int process_array_reverse(int*, int)` |
| 3 | `switch_fallthrough_calculator` | ✅ | ✅ | `src/lib.c:77` | `int switch_fallthrough_calculator(int, int)` |
| 4 | `allocate_and_compute` | ✅ | ✅ | `src/lib.c:99` | `int allocate_and_compute(int, double)` |
| 5 | `foreach_sum` | ✅ | ✅ | `src/lib.c:123` | `int foreach_sum(int*, int)` — uses the `FOREACH` macro |
| 6 | `fallcalc` | ✅ | ✅ | `src/lib.c:135` | the only symbol declared in the public header `include/lib.h` |

**Missing from Rust `.so`: 0.** No stubs were required; every symbol has a real
translated implementation in `translation/src/lib.rs` with
`#[unsafe(no_mangle)] pub extern "C"`.

Note: `safe_double_to_int` .. `foreach_sum` are *not* declared in `include/lib.h`
but the C file does not mark them `static`, so they have external linkage and
are exported by the `.so`. They are therefore part of the ABI surface and are
tested directly through `dlsym`.

## Macros / types with no symbol

These produce no dynamic symbols but must be replicated numerically:

| C construct | value | Rust counterpart |
|---|---|---|
| `#define OCTAL_MASK_1 0777` | 511 | `const OCTAL_MASK_1: c_int = 0o777` |
| `#define OCTAL_MASK_2 0100` | 64 | `const OCTAL_MASK_2: c_int = 0o100` |
| `#define OCTAL_FLAG 0200` | 128 | `const OCTAL_FLAG: c_int = 0o200` |
| `#define OCTAL_BASE 010` | 8 | `const OCTAL_BASE: c_int = 0o10` |
| `FOREACH(item, array, count)` | nested-for macro | inlined loop in `foreach_sum` |
| `typedef struct { int value; double coefficient; } DataPoint;` | `sizeof == 16`, `align == 8` | `#[repr(C)] struct DataPoint` |

## Undefined (imported) symbols

The C `.so` imports only `malloc` and `free` (plus weak CRT/ITM symbols).
The Rust `.so` imports those same two plus the usual Rust `std`/`libunwind`
runtime symbols (`_Unwind_*`, `memcpy`, `mmap64`, …). All are libc/libgcc —
**0 missing/undefined non-libc symbols.** The Rust translation deliberately
calls the C `malloc`/`free` (declared `unsafe extern "C"`) rather than Rust's
allocator so that allocation-failure behaviour matches bit-for-bit.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
configuration is the default (empty) feature set. `--no-default-features`
is therefore equivalent to the default build. Verified by
`grep -A20 '\[features\]' translation/Cargo.toml` returning nothing.

## Binary / driver

Neither `c_src/CMakeLists.txt` (single `add_library(... SHARED src/lib.c)`)
nor `translation/Cargo.toml` (`crate-type = ["cdylib"]`, no `[[bin]]`) builds an
executable, so the "compare stdout of the two binaries" gate is **N/A**.
