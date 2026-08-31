# jansson 2.15.0 C→Rust translation: verification notes

Run everything with `./verify.sh`. Probe test quality with `./mutation_check.sh`.

## 1. Build-time configuration space

`translation/Cargo.toml` has **no `[features]` table** and `c_src/CMakeLists.txt`
exposes **no options**, so there is exactly one build configuration. It is fixed
by `c_src/include/jansson_config.h` + `jansson_private_config.h`:

`JSON_INTEGER_IS_LONG_LONG`, `JSON_PARSER_MAX_DEPTH=2048`,
`INITIAL_HASHTABLE_ORDER=3`, `USE_URANDOM`, `USE_DTOA`/`DTOA_ENABLED`,
`HAVE_ATOMIC_BUILTINS`, `HAVE_SYNC_BUILTINS`, `HAVE_SCHED_YIELD`,
`HAVE_GETTIMEOFDAY`, `HAVE_GETPID`, `HAVE_OPEN`/`CLOSE`/`READ`, `HAVE_SETLOCALE`.

Inside dtoa.c that resolves to: `IEEE_8087`, `IEEE_Arith`, `Pack_32`, `ULLong`,
`USE_BF96`, `Avoid_Underflow`, `INFNAN_CHECK`, `Check_FLT_ROUNDS`,
`Need_Hexdig`; and *not* `NO_HEX_FP`, `No_Hex_NaN`, `NO_STRTOD_BIGCOMP`,
`MULTIPLE_THREADS`, `Honor_FLT_ROUNDS`, `SET_INEXACT`, `USE_LOCALE`,
`Sudden_Underflow`, `ROUND_BIASED`, `Omit_Private_Memory`, `DEBUG`.

`verify.sh` re-derives this (it aborts if a `[features]` table ever appears) and
runs `cargo check --no-default-features --all-targets` for it.

## 2. Test harness

`translation/tests/common/mod.rs` dlopens **both** shared objects with
`libloading` and only ever calls exported symbols — the Rust side is never
called directly, so the `#[no_mangle]` wrappers are under test too.

* C:    `c_src/build/libjansson.so`
* Rust: `translation/target/release/libjansson.so` (override with `JANSSON_RUST_SO`)

Because the crate is `crate-type = ["cdylib"]` only, an integration test has no
dependency edge on the shared object and **`cargo test` does not rebuild it**.
The harness therefore refuses to run if any file under `src/` is newer than the
`.so`. Always build first (`verify.sh` does).

| test file | covers |
|---|---|
| `level1_low.rs`       | utf.c, memory.c, strbuffer.c, version.c, error.c |
| `alloc_funcs.rs`      | `json_{get,set}_alloc_funcs{,2}` (own process + mutex: global state) |
| `level2_hashtable.rs` | hashtable_seed.c, hashtable.c |
| `level3_dtoa.rs`      | dtoa.c (`dtoa_r`, `dtoa`, `freedtoa`, `dtoa_divmax`), strconv.c |
| `gethex.rs`           | `gethex` (hex-float parser) |
| `strtod_unused.rs`    | `strtod__unused` (Gay's strtod, incl. `bigcomp`) |
| `level4_value.rs`     | value.c |
| `level5_dump.rs`      | dump.c (all six entry points × the whole flag space) |
| `level6_load.rs`      | load.c (all six entry points; compares the full `json_error_t`) |
| `level7_pack.rs`      | pack_unpack.c |

81 tests, all passing. Both libraries are seeded with `json_object_seed(0x5eed1234)`
wherever hash order is observable.

## 3. Exported symbol parity

`nm -D --defined-only`: **130 symbols in the C .so, 130 in the Rust .so, no
missing symbols.** Three were missing initially and were added (see below).

## 4. Bugs found in the Rust translation, and fixes

1. **`src/dtoa.rs`: out-of-bounds panic on `PFIVE[k - 1]` when `k == 0`.**
   dtoa.c reaches `pfive[k-1]` with `k == 0` on the `ilim == 0 && j + k >= 0`
   fast path, i.e. it reads the word *in front of* the table. In the C build that
   word is part of the zero alignment padding between `Lhint[2098]` and the
   32-byte-aligned `pfive[27]` in `.data` — verified by dumping
   `libjansson.so`: the 24 bytes before `pfive` are all zero. Added
   `pfive_at()`, which returns 0 for index −1, reproducing the C exactly.
   Covered by the new `dtoa_r_ilim_zero_path_matches` test.

2. **`src/dtoa.rs`: `dtoa_r` results could not be freed.** The `buf == NULL`
   path used a plain `jsonp_malloc`, but dtoa.c returns a pointer *offset into*
   a `Bigint`, which only `freedtoa()` can release. Replaced with an `rv_alloc()`
   that reproduces dtoa.c's layout (`((int *)buf)[-1] == k`) and records
   `dtoa_result`; `nrv_alloc`'s `s0 == NULL` branch now goes through it too.

3. **Missing exports.** Added `dtoa`, `freedtoa` (`src/dtoa.rs`), `gethex`
   (`src/dtoa_hex.rs`, new) and `strtod__unused` (`src/dtoa_strtod.rs`, new,
   with helpers in `src/dtoa_strtod_helpers.rs`).

## 5. Places where the C is undefined behaviour, replicated deliberately

* `pfive[-1]` in `dtoa_r` — see above; reproduced as 0.
* **`bc.rounding` is read uninitialised in `strtod`.** `Check_FLT_ROUNDS` is
  defined but `Honor_FLT_ROUNDS` is not, so the `switch (bc.rounding)` at
  dtoa.c:4721 is compiled while nothing ever assigns `bc.rounding`. The compiled
  C behaves as the `switch` *default* (round-to-nearest), which is the documented
  intent (`Rounding == Flt_Rounds == FLT_ROUNDS == 1`). The Rust initialises
  `bc.rounding = 1` to land on that default. With `= 0` the suite fails, e.g.
  `"4028714796686480.4028714796686480e42"` → `0x4be489b7d78a1dbc` vs C's
  `0x4be489b7d78a1dbb`.

## 6. C entry points that are not NULL-safe (deliberately not probed)

Passing NULL here faults identically in both libraries, so the tests avoid it:

* `json_dump_callback(json, NULL, ...)` — dump.c calls the callback unconditionally.
* `json_unpack_ex` with a NULL output pointer for `i`, `I`, `f`, `F`, `b`, `o`, `O`
  (only `s` and `s%` validate the target).
* `freedtoa(NULL)` — dereferences `((int *)s - 1)`.

## 7. Test-quality evidence

`./mutation_check.sh` perturbs the Rust in 16 places and asserts the suite
notices. All 16 are caught, including `bigcomp` being neutered and `sulp`
degraded, which proves those deep strtod paths are actually executed.

`STRTOD_DIGLIM` is deliberately *not* mutated: changing it only selects between
two paths that are both correctly rounded, so it is an equivalent mutant.

Reaching `bigcomp` at all required generating **exact midpoints between adjacent
doubles** with more than `STRTOD_DIGLIM == 40` significant digits. The test
computes them exactly via `exact_decimal(m, e)` (a small decimal bignum for
`m·2^e`), giving 6294 midpoint literals on top of 27749 general inputs.
