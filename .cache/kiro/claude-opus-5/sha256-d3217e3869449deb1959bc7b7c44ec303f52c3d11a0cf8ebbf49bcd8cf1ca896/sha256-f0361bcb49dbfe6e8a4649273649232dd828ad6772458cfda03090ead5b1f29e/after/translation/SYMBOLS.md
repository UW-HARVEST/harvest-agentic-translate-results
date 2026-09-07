# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C   : `c_src/build/libharvest-work-BTbZII.so`   (CMake, `CMAKE_BUILD_TYPE=""` → `-O0`)
* Rust: `translation/target/release/libmodeselect_lib.so` (`crate-type = ["cdylib"]`)

Reproduce with:

```sh
nm -D --defined-only c_src/build/libharvest-work-BTbZII.so | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libmodeselect_lib.so | awk '{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms   # must be empty
```

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | signature (from `c_src/include/lib.h` + `c_src/src/lib.c`) |
|---|--------|---------|------------|-----------------------------------------------------------|
| 1 | `classify_mode`             | T | T | `int classify_mode(const char *mode)` |
| 2 | `apply_multiplier`          | T | T | `int apply_multiplier(int base, int level)` |
| 3 | `convert_time_factor`       | T | T | `int convert_time_factor(double factor)` |
| 4 | `convert_negative_overflow` | T | T | `int convert_negative_overflow(double value)` |
| 5 | `get_modified_time`         | T | T | `time_t get_modified_time(int offset_days, int offset_hours)` |
| 6 | `hash_time_value`           | T | T | `int hash_time_value(time_t t)` |
| 7 | `modeselect`                | T | T | `int modeselect(int, int, int, int)` — the only symbol in the public header |

**Missing from Rust: 0.** No `#[no_mangle]` wrapper had to be added and no C module
was left untranslated — `c_src/src/lib.c` is the only C source file in
`CMakeLists.txt`, and all 7 of its external functions are implemented and
exported by the Rust `cdylib`. There are no macro-generated symbols in this C
source (no function-defining macros, no `#ifdef` variants).

## Undefined (imported) symbols

The C `.so` imports `printf`, `strcmp`, `time` from glibc.

The Rust `.so` imports `printf` and `time` from glibc (it calls libc `printf`
directly so stdout formatting is byte-identical), implements `strcmp`'s
comparison inline (`cstr_eq`), and additionally imports the usual
`libstd`/unwinder/allocator set (`_Unwind_*`, `malloc`, `memcpy`, `write`, …).

**0 missing / undefined non-libc symbols in Rust** — every undefined symbol in
the Rust `.so` resolves to glibc, libgcc's unwinder, or a weak optional symbol.
Verified: `ldd -r` reports no unresolved symbols for either object.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, so the complete set of feature combinations is the single default
(empty) configuration. `--no-default-features` is therefore identical to the
default build. There is also **no `[[bin]]` target and no `src/main.rs`**, and
`CMakeLists.txt` builds only `add_library(... SHARED)` with no `add_executable`,
so there is no driver binary whose stdout needs comparing; stdout is instead
compared per-call around the `modeselect` FFI call (see `tests/`).
