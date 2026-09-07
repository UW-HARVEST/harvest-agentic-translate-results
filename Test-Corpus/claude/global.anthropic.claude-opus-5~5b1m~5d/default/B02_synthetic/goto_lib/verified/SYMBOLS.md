# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` exported (defined) symbols

`nm -D --defined-only c_src/build/libdriver.so`

```
00000000000012a5 T driver
0000000000001189 T forward_goto_example
00000000000011e8 T open_with_cleanup
```

## Rust `.so` exported (defined) symbols

`nm -D --defined-only translation/target/release/libdriver.so`

```
00000000000119e0 T driver
0000000000011a70 T forward_goto_example
0000000000011ac0 T open_with_cleanup
```

## Parity table

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|---------------|-------|
| 1 | `driver` | yes (T) | yes (T) | declared in `include/goto.h` |
| 2 | `forward_goto_example` | yes (T) | yes (T) | not in header, but external linkage in `src/goto.c` |
| 3 | `open_with_cleanup` | yes (T) | yes (T) | not in header, but external linkage in `src/goto.c` |

`comm -23` of the two sorted symbol-name lists is **empty**: 0 symbols exported by
the C `.so` are missing from the Rust `.so`.

## Undefined (imported) symbols

The Rust `.so` imports only libc symbols, exactly like the C `.so`:

C: `fclose fgets fopen fprintf ferror printf stderr` (+ glibc startup/`__stack_chk_fail`).

Rust: `fclose fgets fopen fprintf ferror printf stderr` (+ Rust runtime glue and
`memcpy`-class libc primitives). No non-libc undefined symbols remain.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. `cargo check --no-default-features` and
`cargo check` are therefore the same build; both are exercised by the
`check_feature_combos.sh` helper.

## Notes on the C source coverage

`c_src` contains exactly one translation unit (`src/goto.c`, 81 lines) and one
public header (`include/goto.h`). All three functions with external linkage are
translated in `translation/src/lib.rs`. No module was skipped, so no additional
translation work was required for symbol parity.

The project builds **only a shared library** (`add_library(driver SHARED ...)`);
there is no binary/driver executable, so the "compare binary stdout" clause of
the completion gate is satisfied vacuously. Stdout/stderr equality is instead
checked directly around the FFI calls (see `tests/`).
