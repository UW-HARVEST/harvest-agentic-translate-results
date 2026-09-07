# SYMBOLS.md — Phase A symbol surface

C `.so`:    `c_src/build/libharvest-work-ZC3UYW.so`
Rust `.so`: `translation/target/release/libfindrep_lib.so`

Derived mechanically via:

```sh
nm -D --defined-only <so> | awk '{print $3}' | sort
```

## Exported symbol table

| # | symbol | C `.so` | Rust `.so` | C source |
|---|--------|---------|------------|----------|
| 1 | `add_to_accumulator`       | T | T | `lib.c:35` |
| 2 | `multiply_with_multiplier` | T | T | `lib.c:41` |
| 3 | `subtract_from_accumulator`| T | T | `lib.c:47` |
| 4 | `divide_multiplier`        | T | T | `lib.c:53` |
| 5 | `process_octal_string`     | T | T | `lib.c:61` |
| 6 | `find_and_replace_char`    | T | T | `lib.c:67` |
| 7 | `validate_and_normalize`   | T | T | `lib.c:74` |
| 8 | `findrep`                  | T | T | `lib.c:99` |

`comm -23 c.txt r.txt` → **empty**. 0 missing symbols.

## Non-exported C entities (intentionally not symbols)

These are `static` in the C and therefore correctly absent from `nm -D` in both
libraries. They are still part of the *observable* surface because they are
mutated by the exported functions and persist across calls.

| C entity | kind | Rust counterpart |
|----------|------|------------------|
| `accumulator`     | `static int = 0` | `ACCUMULATOR: c_int = 0` |
| `multiplier`      | `static int = 1` | `MULTIPLIER: c_int = 1` |
| `operation_count` | `static int = 0` | `OPERATION_COUNT: c_int = 0` |
| `operations[4]`   | `static operation_func[4]` | `OPERATIONS: [OperationFunc; 4]` |
| `operation_func`  | `typedef int(*)(int,int)` | `type OperationFunc` |
| `string_processor`| `typedef void(*)(char*,int)` | `type StringProcessor` (unused in C too) |

**Consequence for testing:** each `.so` owns a private copy of the three
mutable statics. Differential tests MUST issue the identical call sequence to
both libraries from a fresh process, and compare after *every* call — not just
at the end — because divergence in hidden state is only observable through
later return values.

## Undefined (imported) symbols

C imports `memchr`, `strlen`, `strcpy`, `sprintf` from libc. Rust reimplements
these internally (`c_strlen`, `c_strcpy_from`, `c_memchr`,
`format_octal_message`) and imports no non-libc symbols. Verified:

```sh
nm -D --undefined-only translation/target/release/libfindrep_lib.so
```
yields only libc/`GCC_except_table`-class entries.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table and no `[[bin]]`
target, so there is exactly one build configuration (default = empty feature
set) and no driver binary to compare stdout for.
