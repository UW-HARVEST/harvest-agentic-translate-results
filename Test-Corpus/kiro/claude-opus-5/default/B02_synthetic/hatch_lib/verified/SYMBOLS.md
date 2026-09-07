# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-CVCsLD.so
nm -D --defined-only translation/target/release/libhatch_lib.so
```

The C library is built from a single translation unit (`c_src/src/lib.c`, the only
source listed in `c_src/CMakeLists.txt`). It exports 12 `T` symbols. The public
header `c_src/include/lib.h` declares only `hatch`, but the other 11 functions are
non-`static` and therefore part of the dynamic surface too, so all 12 are required.

## Symbol table

| # | C symbol (`nm -D`) | C type | present in Rust `.so` | Rust definition |
|---|--------------------|--------|-----------------------|-----------------|
| 1 | `increment_counter` | T | yes | `increment_counter` (`#[unsafe(no_mangle)] extern "C"`) |
| 2 | `update_accumulator` | T | yes | `update_accumulator` |
| 3 | `apply_operation` | T | yes | `apply_operation` |
| 4 | `add_three` | T | yes | `add_three` |
| 5 | `multiply_add` | T | yes | `multiply_add` |
| 6 | `complex_calc` | T | yes | `complex_calc` |
| 7 | `shift_array_data` | T | yes | `shift_array_data` |
| 8 | `process_pointer_data` | T | yes | `process_pointer_data` |
| 9 | `compute_with_dynamic_memory` | T | yes | `compute_with_dynamic_memory` |
| 10 | `get_time_based_value` | T | yes | `get_time_based_value` |
| 11 | `manipulate_records` | T | yes | `manipulate_records` |
| 12 | `hatch` | T | yes | `hatch` |

## File-scope state (not exported, but behaviourally observable)

| C declaration | Rust counterpart | notes |
|---------------|------------------|-------|
| `static int global_counter` | `static GLOBAL_COUNTER: CGlobal` | `static`, so absent from `nm -D` in both. Observable through `complex_calc`, `hatch`. |
| `static int global_accumulator` | `static GLOBAL_ACCUMULATOR: CGlobal` | Observable through `process_pointer_data`, `hatch`. |

Because this state is per-loaded-library and mutated by `hatch`, every
differential test must keep the C and Rust libraries in lockstep: the same
sequence of state-mutating calls must be issued to both, in the same order.

## Diff result

```
$ comm -23 <(nm -D --defined-only c.so   | awk '{print $3}' | sort) \
           <(nm -D --defined-only rust.so| awk '{print $3}' | sort)
(empty)
```

**0 C symbols missing from the Rust `.so`.**

Undefined symbols in the Rust `.so` are libc / libgcc-unwind imports only
(`malloc`, `free`, `memmove`, `memset`, `time`, `difftime`, `snprintf`, plus the
Rust runtime's `_Unwind_*`, `pthread_*`, `mmap64`, ... ). No non-libc symbol is
undefined. The C `.so` imports the same libc subset it uses.

## Extra symbols exported by the Rust `.so`

The Rust `cdylib` additionally exports mangled Rust runtime/`std` symbols
(`_ZN...`, `_R...`). These are an artifact of `crate-type = ["cdylib"]` linking
`std` and are not part of the C surface; they cannot collide with C names and are
not a correctness concern.

## Build / configuration surface

- `c_src/CMakeLists.txt` declares **only** `add_library(... SHARED src/lib.c)` —
  there is **no binary executable / driver target**, so the "compare C and Rust
  stdout" clause of Phase B is not applicable.
- `translation/Cargo.toml` has **no `[features]` section**, therefore the only
  feature combination is the default (empty) one. `--no-default-features` and the
  default build are the same build; both are still exercised explicitly (see
  `run_feature_combos.sh`).

---

## Verified evidence

```
$ nm -D --defined-only c_src/build/libharvest-work-CVCsLD.so | wc -l
12
$ comm -23 <(nm -D --defined-only c.so    | awk '{print $3}' | sort -u) \
           <(nm -D --defined-only rust.so | awk '{print $3}' | sort -u)
<empty>
$ nm -D --undefined-only rust.so | grep -v '@\|^ *w \|_Unwind_\|__\|_ITM_\|pthread_'
<empty>
```

Encoded as executable checks in `tests/phase_d_symbols.rs`:

| test | what it enforces |
|------|------------------|
| `phase_d_c_symbol_set_is_fully_exported_by_rust` | `nm -D` set difference C \ Rust is empty |
| `phase_d_expected_twelve_public_functions` | the C `.so` still exports exactly the 12 known names (so the diff above cannot pass vacuously if a C source file is dropped) |
| `phase_d_every_c_symbol_is_dlsym_resolvable_from_rust` | each name is really `dlsym`-able out of the Rust `.so`, not just present in `nm` |
| `phase_d_rust_so_has_no_unresolved_non_libc_symbols` | no dangling non-libc import, i.e. no skipped translation unit |
| `phase_d_no_stub_or_panicking_export` | every export returns cleanly on a benign call in a forked child — catches a symbol that exists but is a stub / `unimplemented!()` |
| `phase_d_project_builds_no_binary_executable` | asserts `add_executable` / `[[bin]]` / `src/main.rs` are still absent, so the "compare binaries' stdout" requirement stays genuinely N/A |

**Result: 0 missing symbols, 0 unresolved non-libc symbols, 0 stub exports.**
