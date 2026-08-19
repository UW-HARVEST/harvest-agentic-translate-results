# SYMBOLS.md — dynamic-symbol parity (Phase A / Phase D)

The C translation unit (`c_src/src/main.c`) is a single file whose only
non-`static` definitions are `run` and `main`. Everything else
(`the_house`, `add_floor`, `add_bedrooms`, `add_floor_to_the_house`,
`print_the_house`) is `static` and therefore has no dynamic symbol.

## How the two shared objects are produced

```sh
# C: executable (the deliverable's reference) + shared library (symbol surface)
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .   # -> build/driver
gcc -shared -fPIC -O2 -o build_c/libcdriver.so c_src/src/main.c      # -> build_c/libcdriver.so

# Rust: bin + cdylib from the same `src/imp.rs` translation unit
cargo build --release        # -> target/release/driver, target/release/libdriver.so
```

`src/imp.rs` holds the translation; `src/main.rs` (bin) and `src/lib.rs`
(cdylib, `#[no_mangle] extern "C"` wrappers) both compile that same module, so
the executable and the exported C ABI can never drift apart.

## `nm -D --defined-only` comparison

| # | C symbol | type | Rust `.so` | notes |
|---|----------|------|------------|-------|
| 1 | `main` | `T` | `main` (`T`) | `int main()` — exported by the C `.so` because a `main` in a translation unit compiled `-shared` still has external linkage. `src/lib.rs` re-exports it as `#[no_mangle] pub extern "C" fn main() -> c_int`. |
| 2 | `run`  | `T` | `run`  (`T`) | `void run(int extra_bedrooms)` — `src/lib.rs` re-exports it as `#[no_mangle] pub extern "C" fn run(c_int)`. |

Symbols in C but missing in Rust: **none** (diff is empty).

Static (non-exported) C functions, deliberately not exported by either `.so`
— they are translated as private Rust `fn`s in `src/imp.rs`:

| C `static` symbol | Rust counterpart (private) |
|---|---|
| `static house_t the_house` | `static mut THE_HOUSE: House` |
| `static void add_floor(house_t*)` | `fn add_floor(&mut House)` |
| `static void add_bedrooms(house_t*, int)` | `fn add_bedrooms(&mut House, i32)` |
| `static void add_floor_to_the_house()` | `fn add_floor_to_the_house()` |
| `static void print_the_house()` | `fn print_the_house(&mut dyn Write)` |
| (`scanf("%d")` inside `main`) | `fn scanf_i32_reader(&mut dyn BufRead)` |

Undefined symbols in the Rust `.so` (`nm -D -u`) are all libc / libgcc-unwind
imports (`malloc`, `write`, `read`, `signal`, `memcpy`, `_Unwind_*`, …); there
are **0 missing/undefined non-libc symbols**.

Verified by `tests/symbol_parity.rs`, which runs `nm -D --defined-only` on both
shared objects and asserts the C-side set is a subset of the Rust-side set (and
that every C symbol is resolvable with `dlsym` through `libloading`).
