# SYMBOLS.md — public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C   : `c_src/build/libdriver.so`            (cmake, gcc, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
* Rust: `translation/target/release/libdriver.so` (`crate-type = ["cdylib"]`)

## C `.so` exported symbols (`nm -D --defined-only`)

```
0000000000001175 T driver
0000000000001119 T print_foo
```

The C translation unit `src/driver.c` contains exactly three top-level entities:

| C entity            | linkage             | exported? |
|---------------------|---------------------|-----------|
| `typedef struct {...} foo_t` | type, no symbol | n/a |
| `void print_foo(const foo_t *foo)` | external (no `static`) | **yes** |
| `void driver(unsigned int, unsigned int, bool, int)` | external, declared in `include/driver.h` | **yes** |

Note: `print_foo` is *not* declared in the public header, but it is **not
`static`**, so it has external linkage and is a real part of the `.so`'s ABI
surface. It is therefore verified as a first-class entry point (Phase B/C),
not treated as an internal helper.

## Rust `.so` exported symbols (filtered to non-Rust-runtime symbols)

```
0000000000011710 T driver
0000000000011740 T print_foo
```

## Parity table

| # | symbol      | in C `.so` | in Rust `.so` | status |
|---|-------------|-----------|---------------|--------|
| 1 | `driver`    | T         | T             | ✅ present, `#[no_mangle] extern "C"` |
| 2 | `print_foo` | T         | T             | ✅ present, `#[no_mangle] extern "C"` |

**Symbol diff (C minus Rust): EMPTY.** No symbol required translation work and
no stubs exist — both functions are fully translated in `src/lib.rs`.

Undefined (imported) symbols of the Rust `.so` are libc/Rust-runtime only
(`printf`, `memcpy`, unwinding/personality-free because `panic = "abort"`).
`nm -D -u` shows no missing non-libc symbols.

Reproduce with:

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so            | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort \
       | grep -vE '^(_|rust_|__rust)')
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, so the only build configuration is the default one
(`--no-default-features` is equivalent to the default). This is verified by
`scripts/check_features.sh`, which enumerates features from `Cargo.toml` and
loops over every combination.
