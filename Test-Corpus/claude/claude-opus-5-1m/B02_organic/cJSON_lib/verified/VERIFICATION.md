# VERIFICATION.md — how the C→Rust translation of cJSON 1.7.19 was verified

Everything below is reproducible with:

```sh
./verify.sh          # C build + cargo check + the whole suite, for every feature combination
```

## What is under test

| | |
|---|---|
| C reference | `c_src/cJSON.c` → `c_src/build/libcjson.so.1.7.19` (78 exported symbols) |
| C reference | `c_src/test.c` → `c_src/build/libcJSON_test.so` (exports `driver`) |
| Rust translation | `src/lib.rs` + `src/ffi.rs` + `src/test_driver.rs` → `target/release/libcJSON_test.so` (80 exported symbols) |

Both C `.so`s and the Rust `.so` are loaded **with `libloading` at runtime** and
every call goes through `dlsym`, so the `#[no_mangle] extern "C"` wrappers are
part of the system under test. The Rust crate is never linked into the test
binaries.

Because C and Rust items live on the same heap (both allocate with libc
`malloc`), the tests compare far more than return values: after each operation
the complete item graph is dumped in an address-independent form
(`type`, `valuestring`, `string`, `valueint`, the `valuedouble` **bit pattern**,
and the `child`/`next`/`prev` topology re-expressed as node indices) and the two
dumps must be identical, together with every printed byte and every
`cJSON_GetErrorPtr()` offset.

## Phase A — artifacts

| file | content |
|------|---------|
| `SYMBOLS.md` | all 79 C dynamic symbols, all present in the Rust `.so`; 0 missing; 0 undefined non-libc symbols |
| `ERRORS.md` | 183 rows — every distinct rejection in `cJSON.c`/`test.c`, derived by grepping every `return false` / `return NULL` / `return 0` / `goto fail` / range check / `INT_MAX` / `INT_MIN` / `CJSON_NESTING_LIMIT` / `CJSON_CIRCULAR_LIMIT` / surrogate constant |
| `CONFIGS.md` | 92 rows — the pruned cross-product of every runtime option (`format`, print sink, prebuffer size, parse length/`require_null_terminated`/`return_parse_end`, case sensitivity, key ownership, references, `recurse`, allocator hooks, locale) with every input shape the C code special-cases |

## Phase B / C — the test suite (87 tests, 14 binaries)

| test binary | rows | what it drives |
|-------------|------|----------------|
| `phase_b_core` | C1–C2, C7–C24 | every constructor and the item-building API |
| `phase_b_hooks` | C3–C6, C89, ERRORS 174 | the allocator-hook axis (custom hooks make `global_hooks.reallocate == NULL`, which switches `ensure()`/`print()` to the copy path). Also compares the exact **number of allocations and frees**. |
| `phase_b_print` | C25–C37 | `cJSON_Print`, `PrintUnformatted`, `PrintBuffered` (9 prebuffer sizes × 5 `fmt` values), `PrintPreallocated` (every buffer length from 0 to exact+6, comparing the whole buffer including untouched bytes) |
| `phase_b_parse` | C38–C57 | ~200 documents × 4 entry points × lengths {0,1,strlen,strlen+1,longer} × `require_null_terminated` {0,1,2,-1,INT_MIN} × `return_parse_end` {NULL,ptr}; truncation at *every* offset; BOM; nesting limit 999/1000/1001 |
| `phase_b_api` | C58–C84 | query/mutation/duplicate/compare, incl. all 1024 `type` values × 10 predicates and the `CJSON_CIRCULAR_LIMIT` boundary at depth 9998–10002 |
| `phase_b_pipeline` | C85–C88 | `cJSON_Minify` (whole-buffer comparison), minify→parse, parse→10 random mutations→print (120 rounds), print→parse round trip |
| `phase_b_extra` | C27/C41/C60/C81/C83/C92 | every byte value 1–255 as an object key and probe (`tolower`), embedded NULs from `\u0000` **and** raw NUL bytes, formatted printing at depth 1–999, 800 randomized number comparisons, duplicate keys |
| `phase_b_locale` | C91 | the `ENABLE_LOCALES` path under `C`, `de_DE.utf8`, `fr_FR.utf8` and `ps_AF.utf8` (a **multi-byte** decimal separator that cJSON truncates to one byte and therefore mangles — reproduced identically) |
| `phase_b_driver` | C90, ERRORS 182–183 | `driver` from `test.c`: fd 1 is captured and compared byte-for-byte for the canonical arguments plus 25 randomized argument sets; the `NULL`-string case is run in child processes and both die with `SIGSEGV` |
| `phase_c_errors` | ERRORS 1–99 | accessors, `SetValuestring`, every `ensure()` failure, all UTF-16/escape rejections, parse/print entry points, container syntax errors |
| `phase_c_errors2` | ERRORS 100–183 | query/add/detach/insert/replace/create/duplicate/compare rejections, `cJSON_free(NULL)`, `cJSON_malloc(0)`, `INT_MAX`/`INT_MIN`/NaN saturation, out-of-range `cJSON_bool` values (2, −1, 256, `INT_MIN`, `INT_MAX`) on every boolean parameter, and 22 out-of-range `type` values |
| `phase_c_alloc` | ERRORS 8, 9, 12, 19, 26, 27, 41, 52, 55, 57, 60, 75, 86, 111, 119, 146, 152, 155–157, 177 | a *fail-on-the-Nth-malloc* allocator installed in both libraries; the 66-allocation scenario is replayed with the failure injected at every single allocation index |
| `phase_c_huge` | ERRORS 23, 25 | the two `INT_MAX` guards in `ensure()`, reached with a 2 GiB `cJSON_Raw` payload, plus the `newsize = INT_MAX` growth branch (1 GiB payload) |
| `phase_d_symbols` | Phase D | `nm -D` parity and "no undefined non-libc symbols", re-checked on every run |

All randomized rows use a fixed seed (`xorshift64*`), so failures are
reproducible; on a mismatch `diff()` writes both observation logs to
`$TMPDIR/cjson_diff_<row>.{C,RUST}.txt`.

Because `global_error`, `global_hooks` and the `cJSON_Version` buffer are
process-global inside each library, `diff()` holds a process-wide mutex, and the
hook/locale/allocation-failure tests live in their own test binaries.

## Divergence found and fixed

One real behavioural divergence was found and fixed in the Rust code (the C was
never touched):

* **`cJSON_GetNumberValue` on a non-number returned the wrong NaN.**
  `cJSON.c` returns `(double) NAN`. The reference build compiles with
  `-std=c89 -pedantic`, under which glibc's `<math.h>` does **not** define the
  C99 `NAN` macro, so cJSON's own fallback `#define NAN 0.0/0.0` is what gets
  compiled — and on x86-64 `0.0/0.0` is the "floating-point indefinite" quiet
  NaN, whose **sign bit is set**: `0xFFF8000000000000`. The translation returned
  Rust's `f64::NAN` (`0x7FF8000000000000`).
  Fixed in `src/lib.rs` (`NAN_0_DIV_0`); verified by
  `phase_b_api::c63_c64_value_accessors` and
  `phase_c_errors::rows_1_to_7_accessor_rejections`.

Two independent line-by-line audits of the whole of `cJSON.c` and `test.c`
against the Rust (each with its own additional randomized campaign) reported no
further divergences.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` shows **0** C symbols missing from the Rust
      `.so` and **0** undefined non-libc symbols (enforced by
      `phase_d_symbols`). The Rust `.so` exports one *extra* symbol,
      `cJSON_Duplicate_rec`, which has external linkage in `cJSON.c` and is only
      absent from the C `.so` because of `-fvisibility=hidden`.
- [x] **Phase B** — all **92** `CONFIGS.md` rows pass, each across randomized
      inputs where the row is not a fixed boundary.
- [x] **Phase C** — all **183** `ERRORS.md` rows have a passing differential
      test asserting the *same* sentinel/error code/error pointer. The 14 rows
      that are unreachable through the public API are marked `UNREACHABLE` with
      the reason and with the reachable public-level guard that is tested
      instead.
- [x] **Every feature combination** — `Cargo.toml` declares no `[features]`, so
      there is exactly one combination; `verify.sh` enumerates it mechanically
      (and would enumerate the full power set if features were added) and runs
      `cargo check --all-targets` plus the whole suite for each. The suite also
      passes against the **debug** profile `.so`
      (`CJSON_RUST_SO=target/debug/libcJSON_test.so`), i.e. with Rust's
      overflow checks and debug assertions enabled, so no arithmetic in the
      translation relies on release-mode wrapping.
