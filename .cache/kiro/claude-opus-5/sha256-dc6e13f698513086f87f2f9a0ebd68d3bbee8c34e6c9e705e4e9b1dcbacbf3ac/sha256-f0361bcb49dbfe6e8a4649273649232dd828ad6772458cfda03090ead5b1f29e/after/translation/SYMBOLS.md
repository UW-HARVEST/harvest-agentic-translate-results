# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

* C  `.so`: `c_src/build/libdriver.so`
* Rust `.so`: `translation/target/release/libdriver.so`

## C translation units

The whole library is a single translation unit, `c_src/src/driver.c`
(the only source listed in `c_src/CMakeLists.txt`). There is no second
module that could have been skipped, so no missing-source translation
work was required.

## Symbol table

| # | symbol | C `.so` | Rust `.so` | declared in `include/driver.h`? | notes |
|---|--------|---------|------------|--------------------------------|-------|
| 1 | `driver`    | `T` | `T` | yes | `void driver(unsigned int x, unsigned int y, bool b, int z)` |
| 2 | `print_foo` | `T` | `T` | no (external linkage anyway — not `static`, so it is part of the ABI surface and is the lowest-level entry point) | `void print_foo(const foo_t *foo)` |

`foo_t` is defined only inside `driver.c`; it contributes no symbol.
`printf` is an *undefined* (imported) libc symbol in both objects, not a
defined export, so it is not part of the parity set.

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so         | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
```

Result: **empty** — 0 symbols missing from the Rust `.so`, 0 extra.

## Undefined (imported) symbols in the Rust `.so`

Only libc / runtime symbols (`printf`, and the usual glibc startup and
unwinder entries). 0 missing non-libc symbols.

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
