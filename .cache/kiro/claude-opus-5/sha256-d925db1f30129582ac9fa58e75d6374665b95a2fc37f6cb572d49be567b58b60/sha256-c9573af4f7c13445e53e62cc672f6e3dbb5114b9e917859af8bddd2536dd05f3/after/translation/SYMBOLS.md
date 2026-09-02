# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-KMNxtx.so
nm -D --defined-only translation/target/release/libfindrep_lib.so
```

## C `.so` public symbols vs Rust `.so`

The C translation unit is a single file (`c_src/src/lib.c`). It contains 8
non-`static` functions; the 3 `static int` globals and the `static
operation_func operations[4]` table have internal linkage and are therefore
**not** part of the dynamic symbol table (confirmed: `nm -D` shows no
`accumulator`, `multiplier`, `operation_count`, `operations`).

| # | C symbol | type | present in Rust `.so`? | Rust item |
|---|----------|------|------------------------|-----------|
| 1 | `add_to_accumulator`        | `T` (text, global) | YES | `#[no_mangle] pub unsafe extern "C" fn add_to_accumulator` |
| 2 | `multiply_with_multiplier`  | `T` | YES | `#[no_mangle] pub unsafe extern "C" fn multiply_with_multiplier` |
| 3 | `subtract_from_accumulator` | `T` | YES | `#[no_mangle] pub unsafe extern "C" fn subtract_from_accumulator` |
| 4 | `divide_multiplier`         | `T` | YES | `#[no_mangle] pub unsafe extern "C" fn divide_multiplier` |
| 5 | `process_octal_string`      | `T` | YES | `#[no_mangle] pub unsafe extern "C" fn process_octal_string` |
| 6 | `find_and_replace_char`     | `T` | YES | `#[no_mangle] pub unsafe extern "C" fn find_and_replace_char` |
| 7 | `validate_and_normalize`    | `T` | YES | `#[no_mangle] pub unsafe extern "C" fn validate_and_normalize` |
| 8 | `findrep`                   | `T` | YES | `#[no_mangle] pub unsafe extern "C" fn findrep` |

**Missing from Rust: 0.** No `#[no_mangle]` wrapper had to be added and no C
module was left untranslated — `c_src/src/lib.c` is the only C source file
listed in `c_src/CMakeLists.txt`, and every function in it is implemented in
`translation/src/lib.rs`. No stubs / `unimplemented!()` are used anywhere.

## Header surface

`c_src/include/lib.h` declares only:

```c
int findrep(int a, int b, int c, int d);
```

The other 7 symbols are *not* declared in the public header but **are**
exported with global linkage (they are non-`static` definitions), so they are
part of the ABI a real external caller can reach via `dlsym`. All differential
tests therefore exercise all 8, not just `findrep`.

## Undefined (imported) symbols

C `.so` imports (`nm -D -u`): `memchr`, `sprintf`, `strcpy`, `strlen`
(all `GLIBC_2.2.5`) plus the weak `_ITM_*` / `__cxa_finalize` /
`__gmon_start__` stubs.

The Rust `.so` re-implements `strlen`, `memchr`, `strcpy` and the
`sprintf("%o"/"%d")` rendering internally (`c_strlen`, `c_memchr`,
`c_strcpy_bytes`, `format_octal`) instead of importing them, which is a
permitted implementation difference: no libc symbol appears as an
*unresolvable* undefined reference. Verified with `ldd -r` — 0 unresolved
non-libc/non-`std` symbols in the Rust `.so`.

## Verification commands

```sh
comm -13 \
  <(nm -D --defined-only translation/target/release/libfindrep_lib.so \
      | awk '{print $3}' | sort) \
  <(nm -D --defined-only c_src/build/libharvest-work-KMNxtx.so \
      | awk '{print $3}' | sort)
# -> empty output == 0 C symbols missing from the Rust .so
```

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` section, so the complete
feature-combination set is the single empty combination. `--no-default-features`
and the default build are byte-identical in configuration; both are still
exercised by the automated sweep in `run_all.sh`, which also enumerates the
power set of any features added later:

```
=== feature combinations to verify (2) ===
  default:
  no-default:
  (Cargo.toml declares no [features]; the set is the single empty combination)
```

`run_all.sh` re-checks symbol parity for **every** combination x **both**
profiles (release and debug) and reports:

```
ok      symbol parity: all 8 C symbols exported by the Rust .so
ok      no unresolved undefined symbols
```

for all four (combination, profile) pairs, and the final `diff` of the two
exported-symbol sets prints `identical exported-symbol sets`.
