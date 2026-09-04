# VERIFICATION.md — PCRE2 10.48 C → Rust differential verification

The C in `c_src/` is the ground truth. Every test loads **both** shared libraries
through `libloading` and compares them across the FFI boundary; no Rust function
is ever called directly, so the `#[no_mangle]` export wrappers are exercised too.

```
C    : c_src/build/libpcre2.so                  (cmake, -DSUPPORT_UNICODE, width 8)
Rust : translation/target/release/libpcre2.so   (cdylib, overflow-checks off)
```

The **release** cdylib is the one under test: its profile disables
`overflow-checks`, matching C's wrapping arithmetic. A debug build would panic on
arithmetic that the C performs legally.

## How to run

```sh
./run_tests.sh                 # rebuild both .so files, then the whole suite
./check_features.sh            # every Cargo feature combination (see Phase D)
```

`run_tests.sh` exports nothing special, but the deeply-recursive DFA patterns want
a large stack; the canonical invocation is

```sh
RUST_MIN_STACK=67108864 ./run_tests.sh -- --test-threads=1
```

## Phase A — surface map

| artifact | contents | rows |
|---|---|---|
| `SYMBOLS.md` | every symbol from `nm -D` on the C `.so`, matched against the Rust `.so` | 143 |
| `ERRORS.md` | error-surface table: one row per distinct rejection, derived from the C source and then **executed** against the C `.so` | 411 |
| `CONFIGS.md` | configuration-surface table: one row per meaningful combination of valid options × input shapes | 260 |

### Symbol parity

```
symbols exported by C .so    143
symbols exported by Rust .so 143
missing in Rust               0
extra in Rust                 0
undefined non-libc symbols    0
```

The diff is **empty**. Nothing is stubbed or `unimplemented!()`; every symbol is
backed by a real translation. All undefined symbols in the Rust `.so` are libc /
unwinder imports (`malloc`, `memcpy`, `tolower`, `_Unwind_*`, `__cxa_*`, …).

## Phase B — valid-path differential tests

Driven from the lowest-level exports upward, not only through the convenience
wrappers. Every compile is checked on **four** independent observables:

1. success/failure and the exact `*errorcode` / `*erroroffset`,
2. all 27 `pattern_info` selectors (including the `FIRSTBITMAP` and `NAMETABLE`
   *bytes*, not the pointers),
3. **the entire compiled bytecode, byte for byte**, obtained via
   `pcre2_serialize_encode_8` — this is the strongest available check and catches
   any codegen divergence anywhere in the compiler,
4. then matching: `rc`, the *defined* part of the ovector, `startchar` and `mark`,
   through **both** the interpreter and the DFA.

| file | tests | what it covers |
|---|---|---|
| `tests/lowlevel.rs` | 34 | `_pcre2_*_8` internals: `strlen/strcmp/strncmp/strcpy_c8`, `ord2utf` (exhaustive 0..0x110000 + 20 k beyond Unicode), `valid_utf` (all 256 single bytes, **all 65 536 two-byte pairs**, 110 k randomized), `is_newline`/`was_newline` × all 8 newline types, `ckd_smul`, `memctl_malloc`, `script_run`, `extuni`, `find_bracket`, `get_error_message`, `maketables`, and all 27 exported data tables byte-compared |
| `tests/compile_match.rs` | 22 | 200-pattern corpus × 26 single compile options, UTF/UCP combos, all 6 newline × 2 bsr conventions, all 17 `PCRE2_EXTRA_*`, all 9 match options, DFA shortest/partial/restart-after-partial, DFA workspace sizes, ovector sizes 0…100, every start offset, `PCRE2_ZERO_TERMINATED`, own `maketables()`, match/depth/heap limits, offset limits, varlookbehind & parens-nest limits, pattern-length limits, `set_optimize`, `code_copy`/`code_copy_with_tables`, plus randomized: 4 000 generated patterns, **15 000 raw-byte fuzz patterns**, randomized subjects and randomized UTF subjects (including start offsets landing mid-character) |
| `tests/substitute.rs` | 22 | full `SUBSTITUTE_*` flag matrix, ~180-entry replacement-syntax corpus, buffer sizing incl. the two-pass workflow, UTF case conversion, named/duplicate/unset groups, `SUBSTITUTE_MATCHED`, substitute-callout and substitute-case-callout sequences, 3 400 fuzz iterations |
| `tests/substring.rs` | 13 | every `pcre2_substring_*` entry point across a 51-pattern corpus, buffer sizes, DFA match data, partial matches, `next_match` iteration, 3 200 randomized iterations |
| `tests/serialize.rs` | 10 | encode of 1…30 codes with byte-identical blobs, `get_number_of_codes`, same-library round-trip, **cross-decode** (C blob → Rust decoder and vice versa), custom general contexts, `MIXEDTABLES`, count variants, 400 randomized iterations |
| `tests/convert.rs` | 7 | POSIX-BASIC / POSIX-EXTENDED / GLOB (all four glob spellings) × UTF flags, glob separator × escape matrix, all truncating lengths, 6 000 randomized conversions — and every successful conversion is then re-compiled in both libraries |

## Phase C — error-path differential tests

Every row of `ERRORS.md` has a test. Each asserts the two libraries return the
**same** error code and the same auxiliary output (offset, written length,
rendered message) — not merely "both failed".

| file | tests | `ERRORS.md` sections |
|---|---|---|
| `tests/compile_errors.rs` | 5 | **B** — all 120 `ERR1`…`ERR120` compile errors |
| `tests/api_errors.rs` | 19 | **A** (26), **H** (16), **I** (5), **J** (14) |
| `tests/match_errors.rs` | 33 | **C** (26), **D** (28), **M** (22) |
| `tests/misc_errors.rs` | 16 | **L** (34), **N** (21) |
| `tests/convert.rs` | (shared) | **K** (16) |

Beyond the table, the generic boundaries are swept exhaustively:

* **every one of the 32 bits** of the compile-options word, the extra-options
  word and the match-options word, individually, plus `0xFFFFFFFF`;
* **out-of-range enum values across the FFI boundary** — `pcre2_config_8`,
  `pcre2_pattern_info_8`, `pcre2_set_newline_8`, `pcre2_set_bsr_8`,
  `pcre2_set_optimize_8`, `pcre2_set_glob_separator_8` and
  `pcre2_set_glob_escape_8` are each driven over their whole small-integer range
  and past it (C enums accept any `int`, so these are real inputs);
* null pointers to every entry point that has a defined answer for them;
* zero and oversized lengths; start offsets one past the end and `SIZE_MAX`;
* `BADMAGIC` (junk `code`) and `BADMODE` (valid magic, cleared mode bit) for
  every entry point that validates a code;
* **allocation-failure injection**: a failing allocator is installed and the
  failure point is stepped through *every* allocation index of a compile, a
  `code_copy`, a `code_copy_with_tables`, a `match_data_create`, a `maketables`
  and each context constructor/copier. This also asserts the two libraries make
  the **same number of allocations**, which is a strong structural equivalence
  check in its own right.

### Rows that are undefined behaviour in the C

A number of `ERRORS.md` rows describe inputs the C library does not validate: it
dereferences the argument and crashes. These are **not** called — there is no
"correct" result to compare against, and a crash in both libraries proves
nothing. Each was verified to crash the **C** `.so` too, and is documented in a
dedicated test rather than silently dropped:

* `PCRE2_NO_UTF_CHECK` / `PCRE2_CONVERT_NO_UTF_CHECK` on genuinely invalid UTF-8
  (documented UB; both libraries segfault identically);
* `PCRE2_DFA_RESTART` with a workspace not left behind by a previous PARTIAL
  match — the sanity check at `pcre2_dfa_match.c:3453` is insufficient. The
  documented partial→restart flow **is** tested, including the workspace bytes;
* `pcre2_code_copy_8` / `pcre2_code_copy_with_tables_8` and the
  `pcre2_substring_*_8` family on a junk `code` — neither performs a magic
  check (`pcre2_code_copy` calls `code->memctl.malloc` immediately);
* `pcre2_*_context_copy_8(NULL)`, `pcre2_get_mark_8` /`get_ovector_*` /
  `get_startchar_8` / `next_match_8` with a NULL `match_data`,
  `pcre2_get_error_message_8` with a NULL buffer,
  `pcre2_substitute_8` with `outlengthptr == NULL`;
* `PCRE2_ZERO_TERMINATED` on a buffer with no NUL; `length = SIZE_MAX - 1`;
  a `wscount`/`number_of_codes` larger than the real array; double frees.

For each, the well-defined *neighbour* is tested so the boundary is pinned down.

### Corrections made to `ERRORS.md`

The table was written from the C source and then executed against the C `.so`.
Twelve rows named a representative pattern that actually triggers an *earlier*
error; those rows were corrected to the real trigger (found by brute force), not
weakened:

| row | error | corrected trigger |
|---|---|---|
| 61 | `ERR35` | `(?<=` + `(?\|a\|b)`×2001 + `)x` (the 2000-iteration cap) |
| 62 | `ERR36` | needs `PCRE2_UTF`; compiles without it |
| 74 | `ERR48` | `(?<` + `n`×129 + `>a)` |
| 94 | `ERR68` | `\c` + the literal byte 0x7F (not the text `\x7f`) |
| 99 | `ERR73` | needs `PCRE2_UTF` |
| 102 | `ERR76` | `(*MARK:` + `m`×256 + `)a` |
| 104 | `ERR78` | `\N{U+}` needs `PCRE2_UTF` |
| 110 | `ERR84` | needs `parens_nest_limit` raised, else `ERR19` fires first |
| 113 | `ERR87` | `(?<=` + `a`×70000 + `)b` |
| 145 | `ERR119` | `\g{1x` / `\g{+1x}` |
| 348 | — | `PCRE2_JIT_TEST_ALLOC` is checked *before* the NULL check |
| 397 | — | split: only 6 entry points validate the magic; `+397a` records the ones that do not |

## Phase D — feature combinations and completion gate

`translation/Cargo.toml` declares **no** `[features]` section, and there is not a
single `#[cfg(...)]` gate anywhere in `translation/src/` — so the crate has
exactly one configuration. `check_features.sh` proves this mechanically: it
extracts the feature table from `Cargo.toml`, builds the power set, and runs
`cargo check` plus the full suite for each element (here: the default alone).

The build configuration the whole translation assumes, confirmed at runtime via
`pcre2_config_8`:

| setting | value |
|---|---|
| `PCRE2_CODE_UNIT_WIDTH` | 8 |
| `SUPPORT_UNICODE` | **on** (`config(UNICODE)` → 1) |
| `SUPPORT_JIT` | **off** (`config(JIT)` → 0) |
| `LINK_SIZE` | 2 |

Because JIT is off, the entire `pcre2_jit_*` surface is verified as a set of
*defined failure* results rather than skipped: `jit_compile` → `JIT_BADOPTION`
(or `JIT_UNSUPPORTED` for `TEST_ALLOC` alone), `jit_match` → `JIT_BADOPTION`,
`jit_stack_create` → `NULL`, `INFO_JITSIZE` → 0.

### Completion checklist

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing and 0 undefined non-libc symbols.
- [x] Phase B: every `CONFIGS.md` row passes across randomized inputs.
- [x] Phase C: every `ERRORS.md` row has a passing error-path test (or is
      documented as C-side UB with its defined neighbour tested).
- [x] Phase D: the single feature configuration is verified, and proven to be
      the only one.

### Test-suite hygiene

* All randomization is seeded, so every run is reproducible.
* Uninitialised memory is never compared. Only ovector pairs `0..rc` are read
  (`rc == 0` means "ovector too small, all pairs filled"; `PCRE2_ERROR_PARTIAL`
  defines pair 0 only; any other negative `rc` defines nothing). Output buffers
  are pre-filled with a shared `0xAA` sentinel before being compared wholesale.
* Raw pointer **values** are never compared between the two libraries — pointers
  are converted to offsets, or the pointed-to bytes are compared.
* Tests that install a process-wide failure-injecting allocator or record
  callback invocations in `static` state take `common::global_lock()`, so the
  suite is correct under parallel test threads as well as `--test-threads=1`.

## Results

```
$ RUST_MIN_STACK=67108864 ./run_tests.sh -- --test-threads=1
api_errors      19 passed; 0 failed
compile_errors   5 passed; 0 failed
compile_match   22 passed; 0 failed
convert          7 passed; 0 failed
lowlevel        34 passed; 0 failed
match_errors    33 passed; 0 failed
misc_errors     16 passed; 0 failed
serialize       10 passed; 0 failed
smoke            3 passed; 0 failed
substitute      22 passed; 0 failed
substring       13 passed; 0 failed
                ---
                184 passed; 0 failed
```

Verified stable: 5 consecutive full runs in the default (parallel) mode and one
`--test-threads=1` run, all exit 0. `./check_features.sh` exits 0 with
`ALL FEATURE COMBINATIONS PASSED`.

### Divergences found in the Rust translation: none

No change was required anywhere under `translation/src/`. Every failure
encountered during this work was a defect in the *test* or an inaccuracy in
`ERRORS.md`, and each is recorded above. The translation reproduced the C
byte-for-byte on every input tried, including:

* the full compiled bytecode for ~19 000 distinct patterns × option combinations,
* all 27 exported data tables,
* the **number of heap allocations** performed by a compile and by each `code_copy`
  variant, and the outcome at every injected allocation-failure index.

### Sensitivity (the suite is not vacuous)

Deliberate mutations were introduced into the Rust source and confirmed to be
caught, then reverted (each verified byte-identical to the original afterwards):

| mutated | result |
|---|---|
| `substitute.rs` `pessimistic_case_inflation` `+10`→`+11` | 2 tests fail |
| `substitute.rs` `find_text_end` `nestlevel == 0`→`<= 1` | 3 tests fail |
| `substitute.rs` `*blength = buff_length + extra_needed` `+1` | 18 tests fail |
| `substitute.rs` callout-negative-return `&= !GLOBAL`→`&= !EXTENDED` | 1 test fails |
| `substitute.rs` `$+` `group = top_bracket`→`-1` | 4 tests fail |
| `substring.rs` off-by-one in the `top_bracket` test | 9 tests fail |
| `serialize.rs` dropping the `tables`-field zeroing | 10 tests fail |
