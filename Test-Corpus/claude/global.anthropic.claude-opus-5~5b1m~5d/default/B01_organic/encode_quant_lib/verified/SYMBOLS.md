# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## Build commands

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-jLNDdo.so   (name derives from parent dir)

cd translation && cargo build --release
# -> translation/target/release/libencode_quant_lib.so
```

## C `.so` exported symbols (`nm -D --defined-only`)

| addr | type | symbol |
|------|------|--------|
| `00000000000010f9` | `T` | `encode_quant` |

Total: **1** defined global text symbol.

## Rust `.so` exported symbols (`nm -D --defined-only`)

| addr | type | symbol |
|------|------|--------|
| `00000000000116d0` | `T` | `encode_quant` |

Total: **1** defined global text symbol.

## Parity table

| # | C symbol | present in Rust `.so`? | how exported from Rust | status |
|---|----------|------------------------|------------------------|--------|
| 1 | `encode_quant` | YES | `#[unsafe(no_mangle)] pub extern "C" fn encode_quant` in `src/lib.rs` | OK |

## Missing symbols

**None.** The symbol diff is empty in both directions:

```
diff <(nm -D --defined-only c_src/build/*.so        | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libencode_quant_lib.so \
                                                    | awk '{print $3}' | sort)
# (no output)
```

No implementation was absent, so no additional C module needed translating and
no `#[no_mangle]` wrapper needed adding. No stubs exist in the Rust crate
(`grep -rn 'unimplemented!\|todo!\|panic!' src/` returns nothing).

## Undefined (imported) symbols

The C `.so` imports nothing but the usual glibc/ELF boilerplate
(`__cxa_finalize`, `_ITM_*`, `__gmon_start__`). The Rust `.so` imports only
libc/`std` runtime symbols. **0 missing/undefined non-libc symbols in the Rust
`.so`** — the crate has no external dependency edges of its own; `encode_quant`
is leaf, pure, integer-only code.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. Enumerated mechanically:

```
$ grep -n '^\[features\]' translation/Cargo.toml   # -> no match
```

Therefore "every feature combination" == `{default}`, and the `--no-default-features`
build is byte-identical in surface to the default build (verified in Phase D).
