# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on the built shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

- C `.so`:    `c_src/build/libdriver.so`
- Rust `.so`: `translation/target/release/libdriver.so`

## Defined (exported) dynamic symbols

`nm -D --defined-only` on each library, restricted to the library's own API
symbols (Rust adds no extra public symbols — its exported set is exactly these
five).

| # | symbol | C signature (from `c_src/src/driver.c`) | in C `.so` | in Rust `.so` | status |
|---|--------|----------------------------------------|------------|---------------|--------|
| 1 | `printLine`    | `void printLine(const char *line)` | T | T | present |
| 2 | `printIntLine` | `void printIntLine(int intNumber)` | T | T | present |
| 3 | `bad`          | `void bad(void)`                   | T | T | present |
| 4 | `good`         | `void good(void)`                  | T | T | present |
| 5 | `driver`       | `void driver(void)`                | T | T | present |

Only `driver` is declared in the public header `c_src/include/driver.h`; the
other four have external linkage in `c_src/src/driver.c` and are therefore part
of the `.so` ABI. All five are treated as public entry points and all five are
tested directly through `dlopen`/`dlsym`.

There are no macro-generated symbols, no exported globals, no `static`
functions, no `enum`/`struct`/`typedef` declarations, and no `#ifdef`
conditional compilation anywhere in the C source (verified by grep).

## Symbol diff

```
comm -23 <c defined symbols> <rust defined symbols>   ->  (empty)
```

**0 C symbols are missing from the Rust `.so`.** No wrappers had to be added and
no C module was left untranslated: `c_src/src/driver.c` is the only C source
file in `CMakeLists.txt`, and all five of its functions are implemented in
`translation/src/lib.rs`. No stubs and no `unimplemented!()` exist in the Rust
crate.

## Undefined (imported) symbols

Neither library has an unresolved non-libc dependency.

- C `.so` imports: `printf`, `puts` (glibc) + the usual weak
  `_ITM_*`/`__cxa_finalize`/`__gmon_start__` stubs.
- Rust `.so` imports: `printf`, `puts`, plus the Rust standard library's libc
  and `_Unwind_*` (libgcc) surface — `malloc`, `free`, `memcpy`, `write`,
  `mmap64`, `pthread_key_create`, etc. All are libc / unwinder symbols, so the
  count of missing or undefined **non-libc** symbols is **0**.

Note on `puts`: the C compiler lowers `printf("%s\n", line)` into `puts(line)`,
which is why `puts` appears as an import in the C `.so`. This is a pure
optimization — the bytes written to `stdout` are identical to `printf("%s\n",
…)`, which is what the Rust translation calls. The differential tests compare
the actual bytes written, so this difference in imports is confirmed to be
unobservable rather than assumed to be.

## Completion checklist

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
