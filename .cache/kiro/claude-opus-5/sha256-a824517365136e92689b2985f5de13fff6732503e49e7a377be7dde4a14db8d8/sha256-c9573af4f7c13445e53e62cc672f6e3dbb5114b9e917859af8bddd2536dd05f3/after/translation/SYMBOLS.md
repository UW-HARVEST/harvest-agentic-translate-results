# SYMBOLS.md — Dynamic symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-HgjfRt.so   (CMake project name = parent dir name)

cd translation && cargo build --release
# -> translation/target/release/libmemchra2_lib.so
```

## C source inventory

`c_src` contains exactly one translation unit and one public header:

| file | contents |
|------|----------|
| `c_src/include/lib.h` | `int memchra2(int a, int b, int c, int d);` — the entire public API |
| `c_src/src/lib.c` | `memchra2` plus 8 `static` (internal-linkage) helpers |

The 8 helpers are `static`, so they have **internal linkage** and are
deliberately absent from the C `.so`'s dynamic symbol table. They are therefore
correctly absent from the Rust `.so` as well; exporting them would be a parity
*violation*, not a fix. They are still translated (they exist as private `fn`s
in `translation/src/lib.rs`) and are exercised transitively through `memchra2`:

`memchra`, `process_buffer`, `int_to_float_bits`, `process_strings`,
`safe_sum_array`, `interpret_as_int`, `count_occurrences`, `complex_iteration`.

No C source file was skipped by the translation: `src/lib.c` is the only
`.c` file listed in `c_src/CMakeLists.txt`.

## `nm -D` defined symbols

C `.so` (`nm -D --defined-only`, excluding the linker-synthesised
`_init`/`_fini`/`__bss_start`/`_edata`/`_end` weak+ABI entries):

| # | symbol | type |
|---|--------|------|
| 1 | `memchra2` | `T` (global text) |

Rust `.so` (`nm -D --defined-only`):

| # | symbol | type |
|---|--------|------|
| 1 | `memchra2` | `T` (global text) |

## Diff

```
$ comm -23 <(c_defined_globals) <(rust_defined_globals)
(empty)
```

**Missing from Rust: 0.** The symbol diff is empty.

## Undefined (imported) symbols

The Rust `.so` must not depend on anything unavailable at load time. All
undefined symbols resolve to libc / the Rust runtime shipped statically inside
the cdylib; `ldd` reports no missing objects and `libloading::Library::new`
succeeds in every test, which is the load-time proof.

Non-libc undefined symbols in the Rust `.so`: **0**.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** — therefore the
only build configuration is the default one, and
`--no-default-features` / `--features <combo>` are vacuous here.
`scripts/verify_all.sh` still enumerates the feature list from `Cargo.toml`
and loops over what it finds, so the check is mechanical rather than assumed.

| # | feature combo | symbol parity | Phase B | Phase C |
|---|---------------|---------------|---------|---------|
| 1 | *(default — the only combo; no features declared)* | [x] | [x] | [x] |

## Verification checklist

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with the exact same name.
- [x] No `static` (internal-linkage) C helper is spuriously exported by Rust.
- [x] `nm -D` shows 0 missing / 0 undefined non-libc symbols in the Rust `.so`.
- [x] No C translation unit was left untranslated.
- [x] No symbol is stubbed, faked, or `unimplemented!()`.
