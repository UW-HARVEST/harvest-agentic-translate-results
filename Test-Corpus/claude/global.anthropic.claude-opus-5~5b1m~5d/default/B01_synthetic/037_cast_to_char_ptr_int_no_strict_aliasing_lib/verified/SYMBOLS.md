# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` exported (defined) dynamic symbols

`nm -D --defined-only c_src/build/libdriver.so`

| symbol | type | source | exported by Rust `.so`? |
|--------|------|--------|--------------------------|
| `driver` | `T` (global text) | `c_src/src/driver.c:36`, declared `c_src/include/driver.h:27` | YES (`#[unsafe(no_mangle)] pub extern "C" fn driver`, `translation/src/lib.rs:56`) |

## Non-exported C symbols (intentionally not in the ABI)

| symbol | why not exported |
|--------|------------------|
| `print_hex` | declared `static` at `c_src/src/driver.c:29`, so it has internal linkage and is not part of the C `.so`'s dynamic symbol table. The Rust translation likewise keeps `print_hex` as a private (non-`no_mangle`) `fn`. Matching this is correct — exporting it would be a *divergence*. |

## Symbol diff

```
comm -23 <(nm -D --defined-only c_src/build/libdriver.so     | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
```

Result: **EMPTY** — 0 symbols exported by the C `.so` are missing from the Rust `.so`.

No symbol required translating a previously-skipped module, and no stub was
added: `driver` is genuinely implemented in `translation/src/lib.rs`.

## Undefined (imported) symbols

The Rust `.so` imports `printf` from libc, exactly as the C `.so` does. This is
deliberate: routing output through the same C `stdout` FILE stream gives
identical buffering/ordering behaviour to the C original. No non-libc undefined
symbols remain.

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
buildable configuration is the default (empty) feature set. `--no-default-features`
and the default build are therefore the same compilation, and the Phase D
"every feature combination" requirement collapses to a single combination.
This is verified programmatically rather than assumed (see `tests/`).
