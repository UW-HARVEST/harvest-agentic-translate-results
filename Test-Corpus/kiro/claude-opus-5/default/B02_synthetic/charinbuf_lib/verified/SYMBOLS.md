# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-PFhtb0.so   (name comes from the parent dir name)

cd translation && cargo build --release
# -> translation/target/release/libcharinbuf_lib.so
```

## Translation-unit inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit: `src/lib.c`.
There is no glob, no conditional source list, and no second `.c` file in the
tree (`find c_src -name '*.c'` → `c_src/src/lib.c` only). So there is no C
module that could have been skipped by the translation; the Rust crate covers
`src/lib.c` in full, split across `src/counter.rs`, `src/helpers.rs`,
`src/charinbuf.rs` and `src/cruntime.rs`.

`c_src/include/lib.h` declares only `charinbuf`; the other nine symbols have
external linkage in `lib.c` without a header declaration, so they are part of
the exported ABI even though they are not in the public header. There are no
namespace/prefix macros in the header, so no symbol renaming applies.

## Defined dynamic symbols (`nm -D --defined-only`)

| # | symbol | C `.so` | Rust `.so` | Rust definition site | signature (C) |
|---|--------|---------|------------|----------------------|---------------|
| 1 | `increment_counter`   | T | T | `src/counter.rs`   | `int increment_counter(int)` |
| 2 | `decrement_counter`   | T | T | `src/counter.rs`   | `int decrement_counter(int)` |
| 3 | `multiply_counter`    | T | T | `src/counter.rs`   | `int multiply_counter(int)` |
| 4 | `reset_counter`       | T | T | `src/counter.rs`   | `int reset_counter(int)` |
| 5 | `is_string_empty`     | T | T | `src/helpers.rs`   | `int is_string_empty(const char *)` |
| 6 | `find_char_in_buffer` | T | T | `src/helpers.rs`   | `char *find_char_in_buffer(const char *, size_t, char)` |
| 7 | `create_buffer`       | T | T | `src/helpers.rs`   | `char *create_buffer(const char *)` |
| 8 | `validate_uint16_range` | T | T | `src/helpers.rs` | `int validate_uint16_range(int)` |
| 9 | `apply_operation`     | T | T | `src/helpers.rs`   | `int apply_operation(operation_func, int)` |
| 10 | `charinbuf`          | T | T | `src/charinbuf.rs` | `int charinbuf(int, int, int, int)` |

Symbols exported by C but missing from Rust: **0**.
Symbols exported by Rust but not by C: **0** (the Rust `.so` exports no extra
public symbols — no mangled Rust names leak, and `counter` is `static` in C so
it is correctly absent from both).

`static int counter;` and `typedef int (*operation_func)(int);` have internal
linkage / are types, so they are not expected in `nm -D` for either build.

## Undefined dynamic symbols (`nm -D -u`)

C imports: `free`, `malloc`, `memchr`, `printf`, `puts`, `strcpy`, `strlen`
(plus the usual weak `_ITM_*` / `__cxa_finalize` / `__gmon_start__`).

Rust imports a superset: the same seven libc routines (`free`, `malloc`,
`memchr`, `printf`, `puts`, `strcpy`, `strlen`) plus the symbols the Rust
`std` runtime needs (`_Unwind_*`, `memcpy`, `memset`, `mmap64`, `pthread_key_*`,
`dl_iterate_phdr`, …). All of them resolve from `libc`/`libgcc_s`, i.e. **0
missing / unresolvable non-libc symbols**; `ldd -r` reports no unresolved
references.

Note: `puts` appears as an *import* of the C object because GCC rewrites
`printf("literal\n")` into `puts("literal")`. That is a pure codegen detail —
the bytes written to stdout are identical, which the differential stdout
comparison in Phase B confirms empirically.

## Verification snippet

```sh
diff <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort) \
     <(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort)
# -> empty
```

## Harness validation (mutation check)

Passing tests only mean something if the harness can *fail*. Each mutation below
was applied to the Rust source, `cargo build --release` was re-run, and the full
suite executed; then the source was restored.

| mutation | detected? |
|----------|-----------|
| `validate_uint16_range`: `value > UINT16_MAX` → `value >= UINT16_MAX` | yes |
| `charinbuf` mode 1: `result += 10` → `result += 11` | yes |
| `charinbuf` default: `"Invalid mode: %d"` → `"Invalid Mode: %d"` (stdout only) | yes |
| `increment_counter`: `wrapping_add` → `saturating_add` | yes |
| `find_char_in_buffer`: `target as c_int` → `target as u8 as c_int` | no — and correctly so: `memchr` truncates its `int` back to `unsigned char`, so sign- vs zero-extension is unobservable |

The first run of this check reported **zero** detections, which exposed a real
trap rather than a real pass: `cargo test` does **not** rebuild a `cdylib`
target, so the suite had been loading a stale `.so`. The harness now hard-fails
if `target/*/libcharinbuf_lib.so` is older than any file in `src/` (see
`assert_so_is_fresh` in `tests/common/mod.rs`), and `verify.sh` always runs
`cargo build --release` before `cargo test`.
