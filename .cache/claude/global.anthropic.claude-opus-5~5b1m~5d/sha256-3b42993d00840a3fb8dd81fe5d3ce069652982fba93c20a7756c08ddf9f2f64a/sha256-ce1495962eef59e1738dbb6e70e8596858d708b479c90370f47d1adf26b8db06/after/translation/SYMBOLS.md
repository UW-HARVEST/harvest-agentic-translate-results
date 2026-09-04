# SYMBOLS.md — Symbol parity (Phase A / Phase D)

Artifacts compared:

* C   : `c_src/build/libharvest-work-8RKbJ6.so` (built via `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
* Rust: `translation/target/release/libenvy_lib.so` (`cargo build --release`, `crate-type = ["cdylib"]`)

## `nm -D --defined-only` on the C `.so`

| # | symbol | type | in Rust `.so`? |
|---|--------|------|----------------|
| 1 | `parse_env_numeric`   | T | YES |
| 2 | `init_config_from_env`| T | YES |
| 3 | `perform_operation`   | T | YES |
| 4 | `apply_bit_operations`| T | YES |
| 5 | `envy`                | T | YES |

The C translation unit contains exactly these five external functions
(`c_src/src/lib.c`); `struct ConfigFlags` / `struct ProcessState` / `BUFFER_SIZE`
are types and a macro, so they produce no symbols. `c_src/include/lib.h` declares
only `envy`, but the other four functions are non-`static` in `lib.c` and are
therefore exported too — all four have `#[no_mangle] extern "C"` wrappers in
Rust.

## Diff

```
$ nm -D --defined-only <c.so>  | awk '{print $3}' | sort > /tmp/c.txt
$ nm -D --defined-only <rs.so> | awk '{print $3}' | sort > /tmp/r.txt
$ comm -23 /tmp/c.txt /tmp/r.txt      # in C, missing from Rust
<empty>
```

**Result: 0 symbols missing from the Rust `.so`.** No stubs were used; every
symbol is a real translation of the corresponding C function.

## Undefined (imported) symbols

The Rust `.so` imports only libc entry points, which is what the C code itself
uses: `getenv`, `atoi`, `strchr`, `memcpy`, `printf`, `fprintf`, `snprintf`,
`stderr` (plus the Rust runtime's own `__rust_*` / unwind personality entries and
`memcpy`/`memmove`-class builtins). There are **0 undefined non-libc symbols**
that a consumer would have to provide.

Verification command used (see `tests/symbols.rs`, which re-runs this check as a
test so it cannot silently rot):

```
nm -D --defined-only  <so> | awk '$2=="T"{print $3}'
nm -D --undefined-only <so> | awk '{print $NF}'
```
