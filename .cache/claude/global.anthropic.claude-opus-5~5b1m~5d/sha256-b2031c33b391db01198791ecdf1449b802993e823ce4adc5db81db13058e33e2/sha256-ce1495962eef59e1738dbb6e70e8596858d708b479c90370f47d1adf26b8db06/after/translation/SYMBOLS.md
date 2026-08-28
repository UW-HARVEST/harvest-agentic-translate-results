# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from the built shared objects:

```
c_src/build/libharvest-work-ihIbAd.so     (C, ground truth)
translation/target/release/libmemchra2_lib.so  (Rust)
```

## 1. `nm -D --defined-only` on the C `.so`

```
$ nm -D --defined-only c_src/build/libharvest-work-ihIbAd.so
00000000000013e1 T memchra2
```

The C translation unit (`c_src/src/lib.c`) contains 9 functions, but 8 of them
are declared `static` (internal linkage) and therefore do **not** appear in the
dynamic symbol table:

| C function | linkage | dynamic symbol? |
|---|---|---|
| `memchra` | `static` | no (inlined/local) |
| `process_buffer` | `static` | no |
| `int_to_float_bits` | `static` | no |
| `process_strings` | `static` | no |
| `safe_sum_array` | `static` | no |
| `interpret_as_int` | `static` | no |
| `count_occurrences` | `static` | no |
| `complex_iteration` | `static` | no |
| `memchra2` | external | **yes** |

`c_src/include/lib.h` confirms the public API is exactly one declaration:

```c
int memchra2(int a, int b, int c, int d);
```

## 2. Symbol parity table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `memchra2` | `T` (global text) | `T` (global text) | **match** |

## 3. Symbol diff

```
$ comm -23 <(nm -D --defined-only C.so   | awk '{print $3}' | sort) \
           <(nm -D --defined-only rust.so | awk '{print $3}' | sort)
<empty>
```

**0 symbols missing from the Rust `.so`.**

No C module was skipped by the translation: `c_src` contains exactly one
`.c` file (`src/lib.c`) and one header (`include/lib.h`), and every function in
it — including all 8 `static` helpers — is present in `translation/src/lib.rs`
as a private Rust function. Nothing is stubbed or `unimplemented!()`.

| C definition (lib.c) | Rust counterpart (src/lib.rs) |
|---|---|
| `static int memchra(const char*, int, size_t)` | `fn memchra(&[u8], c_int, usize) -> c_int` |
| `static int process_buffer(char*, size_t)` | `fn process_buffer(Option<&[u8]>, usize) -> c_int` |
| `static float int_to_float_bits(int)` | `fn int_to_float_bits(c_int) -> f32` |
| `static int process_strings(char**, int, const char*)` | `fn process_strings(Option<&[Option<&[u8]>]>, c_int, &[u8]) -> c_int` |
| `static int safe_sum_array(int*, size_t)` | `fn safe_sum_array(Option<&[c_int]>, usize) -> c_int` |
| `static int interpret_as_int(unsigned char*, size_t)` | `fn interpret_as_int(Option<&[u8]>, usize) -> c_int` |
| `static int count_occurrences(const char*, char)` | `fn count_occurrences(Option<&[u8]>, u8) -> c_int` |
| `static int complex_iteration(int*, size_t)` | `fn complex_iteration(Option<&[c_int]>, usize) -> c_int` |
| `snprintf(buf, sizeof buf, ...)` (libc) | `fn snprintf_into(&mut [u8], &str)` |
| `int memchra2(int,int,int,int)` | `#[unsafe(no_mangle)] pub extern "C" fn memchra2` |

## 4. Undefined (imported) symbols

The Rust `.so` must not require any non-libc symbol that the C `.so` does not.

```
$ nm -D --undefined-only translation/target/release/libmemchra2_lib.so
```

Only the standard glibc / libgcc runtime imports appear (see
`tests/symbol_parity.rs::rust_so_has_no_unresolved_non_libc_symbols`, which
asserts this programmatically). 0 unresolved non-libc symbols.

## 5. Feature combinations

`translation/Cargo.toml` declares exactly two `[features]` keys:

| feature | default? | effect on the exported symbol set |
|---|---|---|
| `default` | — | empty; identical to `--no-default-features` |
| `test_internals` | **no** | adds 9 test-only `harness_*` wrappers around the translations of `lib.c`'s `static` helpers (used by Phase C, see `ERRORS.md`). `memchra2` is unchanged. |

So the complete combination set is:

```
--no-default-features
--no-default-features --features test_internals
--all-features                     (== test_internals)
```

Symbol parity in each combination (checked by `run_verification.sh`, which diffs
`nm -D` per combination, and by `tests/symbol_parity.rs`):

| combination | profile | C symbols missing from Rust | extra Rust symbols |
|---|---|---|---|
| `--no-default-features` | dev | 0 | none (sets are *identical*) |
| `--no-default-features --features test_internals` | dev | 0 | the 9 documented `harness_*` |
| `--all-features` | dev | 0 | the 9 documented `harness_*` |
| `--no-default-features` | release | 0 | none (sets are *identical*) |
| `--no-default-features --features test_internals` | release | 0 | the 9 documented `harness_*` |
| `--all-features` | release | 0 | the 9 documented `harness_*` |

`tests/feature_matrix.rs` additionally fails if a new feature is added to
`Cargo.toml` without extending the verification matrix.
