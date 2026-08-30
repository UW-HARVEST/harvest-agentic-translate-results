# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` exported (dynamic, defined) symbols

`nm -D --defined-only c_src/build/libdriver.so`

| symbol | type | source | present in Rust `.so`? |
|--------|------|--------|------------------------|
| `driver` | `T` (global text) | `c_src/src/driver.c:72` — `void driver(const char *in)` | YES (`#[unsafe(no_mangle)] pub unsafe extern "C" fn driver`) |
| `run`    | `T` (global text) | `c_src/src/driver.c:50` — `void run(house_t *the_house, int extra_bedrooms)` | YES (`#[unsafe(no_mangle)] pub unsafe extern "C" fn run`) |

**Missing from Rust `.so`: NONE.** The symbol diff is empty.

## Rust `.so` exported (dynamic, defined) symbols

`nm -D --defined-only translation/target/release/libdriver.so`

| symbol | type |
|--------|------|
| `driver` | `T` |
| `run`    | `T` |

No extra symbols are exported by Rust (no Rust-mangled leakage, no
`rust_eh_personality`, etc.), so the surfaces are identical in both directions.

## C `static` (file-local, NOT exported) functions

These are `t` (local text) in the C `.so` and therefore not part of the ABI
surface. They are translated as private Rust `fn`s and are exercised
indirectly through `driver` / `run`.

| C symbol | nm type | Rust counterpart |
|----------|---------|------------------|
| `add_floor`    | `t` | `fn add_floor(&mut house_t)` |
| `add_bedrooms` | `t` | `fn add_bedrooms(&mut house_t, c_int)` |
| `print_house`  | `t` | `fn print_house(&house_t)` |
| `parse_val`    | `t` | `fn parse_val(*const c_char, &mut c_int) -> bool` |

## Undefined (imported) symbols

| symbol | C `.so` | Rust `.so` | note |
|--------|---------|------------|------|
| `printf`            | U | U | Rust calls libc `printf` directly, so `%d` / `%.1f` formatting is byte-identical by construction. |
| `puts`              | U | (not used) | GCC rewrites `printf("An error occurred\n")` into `puts("An error occurred")` even at `-O0`. This is a pure libc-level optimisation: the bytes written to stdout are identical (`puts` appends the newline). Rust emits the `printf` call. **Behaviourally equivalent, verified by the differential tests.** |
| `strtol`            | U | U | Rust calls libc `strtol` so parsing/`errno` semantics are identical. |
| `__errno_location`  | U | U | glibc `errno` accessor; the C `errno` macro expands to `*__errno_location()`. |

There are **0 missing / unresolved non-libc symbols** in the Rust `.so`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
build configuration is the default one (`--no-default-features` is equivalent).
Phase D's "repeat for every feature combination" reduces to the single default
combination; this is verified by `check_features.sh`.
