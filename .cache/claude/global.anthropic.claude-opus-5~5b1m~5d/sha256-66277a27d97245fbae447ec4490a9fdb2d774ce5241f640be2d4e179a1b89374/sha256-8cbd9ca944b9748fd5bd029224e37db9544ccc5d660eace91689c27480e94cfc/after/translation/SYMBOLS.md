# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects
(`w`/`V` weak/unique entries filtered — those are toolchain artifacts, not API).

## C `.so` (`c_src/build/libdriver.so`)

```
$ nm -D --defined-only c_src/build/libdriver.so | grep -v ' [wV] '
0000000000001109 T driver
```

## Rust `.so` (`translation/target/release/libdriver.so`)

```
$ nm -D --defined-only translation/target/release/libdriver.so | grep -v ' [wV] '
0000000000011700 T driver
```

## Parity table

| # | symbol | type | in C `.so` | in Rust `.so` | status |
|---|--------|------|-----------|---------------|--------|
| 1 | `driver` | `T` (text, global) | yes | yes | OK — exported by `#[unsafe(no_mangle)] pub extern "C" fn driver` |

**Missing from Rust: none.** The C library is a single translation unit
(`c_src/src/driver.c`) declaring a single public function in
`c_src/include/driver.h`:

```c
void driver(int x, int y);
```

No module of the C source was skipped, so no additional translation work
is required for symbol completeness.

## Undefined (imported) symbols

The C `.so` imports `puts` (gcc rewrites `printf("literal\n")` → `puts`).
The Rust `.so` imports `printf` from libc directly. Both are libc symbols
resolved from `libc.so.6` at load time; neither is a non-libc undefined
symbol.

```
$ nm -D --undefined-only c_src/build/libdriver.so   # puts, plus glibc glue
$ nm -D --undefined-only translation/target/release/libdriver.so  # printf, plus glibc glue
```

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.

## Result

`tests/symbols.rs :: phase_d_symbol_parity` performs this diff at test time
(`comm -23` equivalent over `nm -D` output) and passes:

```
== symbol parity ==
C-only symbols (must be empty):
                       <-- empty
```

Verified under all six profile × feature combinations by `./run_all_combos.sh`.
