# SYMBOLS.md — Phase A: exported-symbol surface

## Build commands used

```
# C shared library
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# => c_src/build/libtranslated_rust.so   (project name comes from the parent dir)

# Rust shared library (crate-type = ["cdylib"], lib name = fallcalc_lib)
cd translated_rust && cargo build --offline --no-default-features
# => target/debug/libfallcalc_lib.so
```

## C source inventory (completeness check)

`c_src` contains exactly **one** translation unit and one public header:

| C file | translated to | status |
|--------|---------------|--------|
| `c_src/src/lib.c` (176 lines) | `src/lib.rs` | fully translated |
| `c_src/include/lib.h` (declares `fallcalc` only) | n/a (declaration only) | n/a |

No C module/file was skipped, so no missing-module translation work was required
in Phase A/D.

## `nm -D --defined-only` comparison

| # | symbol (C `.so`) | kind | in Rust `.so`? | Rust definition |
|---|------------------|------|----------------|-----------------|
| 1 | `safe_double_to_int`           | T (global text) | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C" fn safe_double_to_int` |
| 2 | `process_array_reverse`        | T | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn process_array_reverse` |
| 3 | `switch_fallthrough_calculator`| T | YES | `#[unsafe(no_mangle)] pub extern "C" fn switch_fallthrough_calculator` |
| 4 | `allocate_and_compute`         | T | YES | `#[unsafe(no_mangle)] pub extern "C" fn allocate_and_compute` |
| 5 | `foreach_sum`                  | T | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn foreach_sum` |
| 6 | `fallcalc`                     | T | YES | `#[unsafe(no_mangle)] pub extern "C" fn fallcalc` |

No macro-generated symbols exist in the C source (`FOREACH` is a statement
macro, the `OCTAL_*` macros are integer constants — none of them define symbols).

### Non-symbol entities intentionally NOT exported

* `DataPoint` (`typedef struct`) — a type, emits no dynamic symbol. Present in
  Rust as `#[repr(C)] struct DataPoint`.
* `OCTAL_MASK_1` (0777), `OCTAL_MASK_2` (0100), `OCTAL_FLAG` (0200),
  `OCTAL_BASE` (010) — object-like macros, emit no symbols. Present in Rust as
  `const` items.

## Undefined (imported) symbols

| symbol | C `.so` | Rust `.so` | note |
|--------|---------|-----------|------|
| `malloc@GLIBC_2.2.5` | U | U | Rust deliberately calls libc `malloc` so allocation-failure and `malloc(0)` behaviour is identical. |
| `free@GLIBC_2.2.5`   | U | U | idem |
| `_ITM_*`, `__gmon_start__`, `__cxa_finalize` | w (weak) | w / n-a | toolchain boilerplate, not part of the API surface |

The Rust `.so` additionally imports the usual Rust-std/libc set
(`memcpy`, `pthread_*`, `__rust_*`, …). Those are libc/runtime symbols, not C
API symbols, and are excluded from the parity requirement.

## Phase D symbol-diff result

```
comm -23 <(c defined-only globals) <(rust defined-only globals)   ->  EMPTY
```

**0 missing symbols, 0 undefined non-libc symbols.** See
`tests/phase_d_symbols.rs`, which recomputes this diff with `nm -D` at test
time and fails if it is ever non-empty.
