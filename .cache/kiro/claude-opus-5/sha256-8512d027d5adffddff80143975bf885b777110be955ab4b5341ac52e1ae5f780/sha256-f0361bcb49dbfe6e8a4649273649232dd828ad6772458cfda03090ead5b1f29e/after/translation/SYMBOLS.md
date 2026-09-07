# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

- C `.so`: `c_src/build/libharvest-work-DtYVZa.so`
- Rust `.so`: `translation/target/release/libhalf2float_lib.so`

## C exported (defined) dynamic symbols

Command: `nm -D --defined-only c_src/build/libharvest-work-DtYVZa.so`

```
00000000000010f9 T half2float
```

## Rust exported (defined) dynamic symbols

Command: `nm -D --defined-only translation/target/release/libhalf2float_lib.so`

```
0000000000013810 T half2float
```

## Parity table

| # | symbol | in C `.so` | in Rust `.so` | action |
|---|--------|-----------|--------------|--------|
| 1 | `half2float` | yes (`T`) | yes (`T`) | none — exported via `#[unsafe(no_mangle)] pub extern "C"` |

**Symbol diff: EMPTY.** No symbol exported by the C `.so` is missing from the
Rust `.so`. No stubs were added; the single symbol has a real translated body.

## Whole-module completeness check

`c_src` contains exactly two source files:

- `c_src/include/lib.h` — declares only `float half2float(uint16_t h);`
- `c_src/src/lib.c` — 376 lines: three `static` lookup tables
  (`m__mantissa[2048]`, `m__offset[64]`, `m__exponent[64]`) and the single
  function `half2float`.

`CMakeLists.txt` compiles only `src/lib.c` into the shared library and declares
no `add_executable`. Therefore no C module was skipped by the translation: the
Rust crate contains all three tables and the one function. The three `static`
tables are file-local in C (`static`, so not exported) and correspondingly
private in Rust (`static M__MANTISSA/M__OFFSET/M__EXPONENT`), which is why they
appear in neither `nm -D` listing. This is correct parity, not a gap.

## Undefined symbols (informational)

The C `.so` has 4 undefined symbols, all weak CRT hooks
(`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` additionally imports libc and libgcc-unwind symbols
(`malloc`, `memcpy`, `_Unwind_*`, `pthread_key_create`, …) pulled in by the Rust
standard library / panic runtime. **0 undefined non-libc symbols** — every
non-libc/non-unwind reference resolves inside the object. This difference is an
artifact of linking `std`, not a missing translation unit.

## Verification record

`comm -23` on the two sorted `nm -D --defined-only` name lists:

```
MISSING FROM RUST: (none)  -> symbol diff EMPTY
```

Undefined non-libc / non-unwind symbols in the Rust `.so`: **0**.

Symbol presence was re-checked per build configuration by
`tools/check_all_configs.sh`, which asserts ` T half2float` in
`target/<profile>/libhalf2float_lib.so` for every profile and feature
combination before running the tests.

## Table provenance

The three lookup tables are not hand-copied. `tools/gen_lib_rs.py` parses them
out of `c_src/src/lib.c` with a regex on the `static` declarations and emits
`translation/src/lib.rs`, so the Rust tables cannot drift from the C. The
extraction is independently re-checked by diffing the hex literals of each
table between the two sources:

```
mant: IDENTICAL   (2048 entries both sides)
off:  IDENTICAL   (64 entries both sides)
exp:  IDENTICAL   (64 entries both sides)
```
