# VERIFICATION.md — completion gate

Differential verification of the Rust translation of jansson 2.15.0 against the
original C. Both are built as shared libraries and loaded side by side through
`libloading`; **no test ever calls a Rust function directly**, so the
`#[no_mangle]` / `extern "C"` export wrappers are under test too.

## How to reproduce

```sh
cd translation
./run_tests.sh          # rebuild both .so's, run the whole suite
./run_all_features.sh   # the same, once per cargo feature combination
./mutation_sweep.sh     # prove the suite detects a wrong translation
```

`--test-threads=1` is mandatory and is applied by the scripts: the C `dtoa()`
keeps a process-global, non-thread-safe freelist, and `json_set_alloc_funcs*`
mutates process-global allocator hooks, so concurrent tests would race inside the
C library itself.

`cargo` runs `--offline` because the crates.io index is unreachable from this
sandbox; `libloading 0.8.9` is already present in the local registry cache.

## Gate

- [x] **`SYMBOLS.md`: 0 missing symbols.** `nm -D --defined-only` reports **130**
      symbols for the C `.so` and **130** for the Rust `.so`, with an **empty**
      diff in both directions. Enforced as a test
      (`tests/d_symbols.rs::d_symbol_parity_is_exact`) and additionally every
      symbol is checked to resolve through `dlsym`
      (`::d_every_symbol_is_dlsym_resolvable`).
- [x] **Phase B: every row of `CONFIGS.md` passes** — 192 rows, all `[x]`, none
      open. Driven with randomized inputs from a fixed-seed SplitMix64 PRNG, and
      exhaustively where the domain is small enough (all 256 byte values for
      `utf8_check_first`; every codepoint `0..=0x10FFFF` for `utf8_encode`; all
      2-byte sequences for `utf8_check_full`; every `JSON_INDENT(n)` and
      `JSON_REAL_PRECISION(p)` for `n,p in 0..=32`; the full 32-element decode
      flag lattice; every `json_dumpb` buffer size up to the required length;
      every `json_dump_callback` abort index).
- [x] **No binary executable exists**, so the "C and Rust stdout match" clause is
      not applicable: `c_src/CMakeLists.txt` contains only `add_library(jansson
      SHARED ...)` (no `add_executable`), and `Cargo.toml` declares no `[[bin]]`.
      The library's own I/O entry points are still compared byte-for-byte —
      `json_dumpf` / `json_dumpfd` / `json_dump_file` output is diffed as file
      bytes, and `json_dump_callback` is compared at *chunk-boundary* granularity,
      not just on the concatenated result.
- [x] **Phase C: every row of `ERRORS.md` has a passing error-path test** — all
      rows `[x]`; 22 rows are marked `n/a` with a stated reason (C `assert()`s the
      C itself treats as unreachable, or genuine UB such as calling a NULL
      callback or a `key_len` that would be read before the guard runs). Rows are
      compared on the **full `json_error_t`** — `line`, `column`, `position`,
      `source`, message text, and the `json_error_code` byte at `text[159]` — not
      merely on "both failed".
- [x] **All of the above hold under every feature combination.** `Cargo.toml`
      declares no `[features]` and `src/` contains no `cfg(feature = ...)`, so the
      feature powerset is the single empty set; `run_all_features.sh` proves this
      mechanically and runs the suite under `(default)`, `--no-default-features`
      and `--all-features`. All three: **PASS**.

## Result

**146 tests, 0 failures**, and **0 behavioural divergences found** between the C
and the Rust. No changes were needed in `translation/src/` — every fix in this
session was to a *test* that had encoded a wrong expectation about the C.

Nothing in `c_src/` was modified.

## Why the result is trustworthy

A green suite is only meaningful if a wrong translation would turn it red.
`mutation_sweep.sh` injects one off-by-one / wrong-constant bug at a time into
`translation/src/`, rebuilds the `.so`, runs the whole suite, and requires it to
fail — then restores the source. Current result: **14 mutations injected, 14
caught, 0 escaped**, spanning `dump.rs`, `utf.rs`, `value.rs`, `load.rs`,
`hashtable.rs`, `strbuffer.rs` and `strconv.rs`:

| mutation | caught |
|---|---|
| `dump`: `JSON_ENSURE_ASCII` boundary `0x7F` → `0x80` | yes |
| `dump`: control-char cutoff `0x20` → `0x1f` | yes |
| `utf`: surrogate upper bound `0xDFFF` → `0xDFFE` | yes |
| `utf`: max codepoint `0x10FFFF` → `0x10FFFE` (check) | yes |
| `utf`: max codepoint `0x10FFFF` → `0x10FFFE` (encode) | yes |
| `value`: `json_array_get` bound `>=` → `>` | yes |
| `value`: `json_array_insert_new` bound `>` → `>=` | yes |
| `load`: depth limit `>` → `>=` | yes |
| `hashtable`: `INITIAL_HASHTABLE_ORDER` 3 → 4 | yes |
| `hashtable`: rehash threshold `>=` → `>` | yes |
| `strbuffer`: `STRBUFFER_MIN_SIZE` 16 → 32 | yes |
| `strbuffer`: `STRBUFFER_FACTOR` 2 → 3 | yes |
| `strconv`: exponent switch `decpt > 16` → `> 15` | yes |
| `strconv`: exponent switch `decpt <= -4` → `<= -3` | yes |

The `json_array_get` mutation initially **escaped**, because `table[entries]` in a
freshly-`malloc`ed array is usually already zero, so an out-of-range read returns
`NULL` and looks correct. `tests/b_value.rs::e7_one_past_end_index_with_live_stale_slot`
now plants a *live, non-NULL* `json_t*` in that slot first (append, keep a
reference, then remove — `json_array_remove` shrinks `entries` but leaves the
stale pointer behind), which makes the off-by-one observable.
`::d3_object_lookup_after_delete_with_live_values` does the same for objects.

The OOM suite is likewise self-checking:
`tests/c_errors_oom.rs::zz_injection_harness_is_effective` proves the budgeted
allocator really does force failures (and that the failure point moves with the
budget), so those tests cannot pass vacuously.

## Test layout

| file | tests | covers |
|---|---|---|
| `tests/common/mod.rs` | — | the harness: typed `libloading` wrappers for all 130 symbols, ABI mirrors of `json_error_t` / `strbuffer_t` / `hashtable_t` / `json_t`, a library-independent `Node` tree builder, and a fixed-seed SplitMix64 PRNG |
| `tests/smoke.rs` | 2 | both `.so`s load; version and a trivial round-trip agree |
| `tests/b_internals.rs` | 28 | CONFIGS I–N: `hashtable_*`, `strbuffer_*`, `utf8_*`, `jsonp_dtostr`/`jsonp_strtod`/`dtoa`, memory, error, version |
| `tests/b_encode.rs` | 15 | CONFIGS A: the whole encode-flag surface, `json_dumpb`/`dumpf`/`dumpfd`/`dump_file`/`dump_callback` |
| `tests/b_decode.rs` | 13 | CONFIGS B, C: the 32-flag decode lattice over hand-built, random and *mutated* documents; all five decoders; round-trip |
| `tests/b_value.rs` | 22 | CONFIGS D, E, F: object/array/scalar op-scripts, equality, copying, `json_sprintf` |
| `tests/b_packunpack.rs` | 12 | CONFIGS G, H: every pack/unpack format character and modifier, `JSON_VALIDATE_ONLY`/`JSON_STRICT`, refcount deltas |
| `tests/c_errors.rs` | 35 | ERRORS: every non-OOM rejection, by exact sentinel and `json_error_code` |
| `tests/c_errors_oom.rs` | 13 | ERRORS: every allocation-failure row, via a budgeted allocator swept from budget 0 upwards, in both the 2-arg and 3-arg `json_set_alloc_funcs` modes |
| `tests/d_symbols.rs` | 6 | Phase D: symbol parity, `dlsym` resolvability, `dtoa_r`, `strtod__unused`, `gethex`, `jsonp_error_vset` |

## Notable C behaviours confirmed and replicated

Recorded here because each is a plausible place for a translation to "fix" the C
and thereby diverge:

- `JSON_PRESERVE_ORDER` (0x100) is referenced by **no** `.c` file — a true no-op.
- `JSON_REAL_PRECISION(p)` for `p` in `25..=31` makes **every** real fail to
  encode (`jsonp_dtostr` gives `dtoa_r` a 25-byte scratch buffer).
- `JSON_EMBED` applies to the **root only** (the bit is stripped before
  recursion), and an *empty* embedded container emits nothing at all.
- `json_dumpb` with a short buffer does **not** produce a clean prefix: a chunk
  that does not fit is skipped while `used` still advances, so a later shorter
  chunk can still be written. The tests compare the whole buffer byte-for-byte.
- `utf8_encode` **accepts** UTF-16 surrogates; only `utf8_check_full` rejects them.
- Decoder underflow (`1e-400`) is silently accepted as `0.0`; only overflow errors.
- `error_set` rewrites `json_error_invalid_syntax` to
  `json_error_premature_end_of_input` when the saved text is empty, and omits the
  `near '...'` suffix once the saved text exceeds 20 bytes.
- A *direct* self-reference cannot be built through the public API (the C rejects
  `json == value`), so the cycle tests use indirect 2-, 3- and nested cycles.
- `hashtable_seed` is an exported **global** (`volatile uint32_t`), not a
  function, and `json_object_seed` is one-shot — later calls are ignored.
- `fopen("/", "rb")` succeeds on Linux, so `json_load_file("/")` yields a parse
  error rather than `json_error_cannot_open_file`.
- Overwriting the 8th key of an object triggers a rehash even though `size` does
  not grow, and `hashtable_clear` does **not** shrink the bucket array back.
