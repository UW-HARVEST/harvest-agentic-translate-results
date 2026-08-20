# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## How the two shared libraries are built

`c_src/CMakeLists.txt` links `src/main.c src/analyzer.c src/tokenizer.c` into the
`driver` **executable**, so there is no ready-made `.so` target.  The shared
library used for differential testing is built from *exactly the same three
translation units* (nothing in `c_src/` is modified):

```sh
cd translated_rust/c_src && mkdir -p build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .   # ./driver
gcc -shared -fPIC -O2 -Iinclude -o build/libtextanalyzer_c.so \
    src/tokenizer.c src/analyzer.c src/main.c
```

The Rust side gains a `[lib] crate-type = ["cdylib"]` target
(`target/debug/libtext_analyzer.so`) whose `src/ffi.rs` exports the same C ABI.
`src/main.rs` (the `driver` binary) and `src/ffi.rs` both drive the *same*
`src/driver.rs`, `src/analyzer.rs`, `src/tokenizer.rs`, `src/cio.rs` code, so the
executable and the shared library cannot drift apart.

Because `main.c` is part of the library, `main` and the other non-`static`
functions of `main.c` (`print_menu`, `print_analysis_result`,
`interactive_tokenizer`, `read_file`) are part of the compared surface too.

## `nm -D --defined-only` comparison

| # | C symbol (`libtextanalyzer_c.so`) | source | exported by `libtext_analyzer.so` |
|---|-----------------------------------|--------|-----------------------------------|
| 1 | `analyze_text` | analyzer.c | yes |
| 2 | `analyzer_init` | analyzer.c | yes |
| 3 | `calculate_complexity_score` | analyzer.c | yes |
| 4 | `find_patterns` | analyzer.c | yes |
| 5 | `print_token_distribution` | analyzer.c | yes |
| 6 | `get_tokenizer_ops` | tokenizer.c | yes |
| 7 | `tokenizer_get_stats` | tokenizer.c | yes |
| 8 | `tokenizer_load_text` | tokenizer.c | yes |
| 9 | `tokenizer_next_token` | tokenizer.c | yes |
| 10 | `tokenizer_peek_token` | tokenizer.c | yes |
| 11 | `tokenizer_reset` | tokenizer.c | yes |
| 12 | `interactive_tokenizer` | main.c | yes |
| 13 | `main` | main.c | yes |
| 14 | `print_analysis_result` | main.c | yes |
| 15 | `print_menu` | main.c | yes |
| 16 | `read_file` | main.c | yes |

`comm -23 c_syms rust_syms` (symbols the C `.so` exports that the Rust `.so`
does not) is **empty**: 16/16.

Symbols only the Rust `.so` exports (a superset is allowed; the C `.so` needs no
counterpart because `fflush(NULL)` from the test process drains the C `FILE`
buffers, while the Rust translation owns its own emulated `stdout` buffer):

| extra Rust symbol | purpose |
|-------------------|---------|
| `text_analyzer_flush_stdout` | drains the emulated `stdout` buffer of `cio::Out` from a test, i.e. the analogue of `fflush(NULL)`; the library also registers it with `atexit`, matching C's flush-at-exit |

`nm -D --undefined-only libtext_analyzer.so` lists only `GLIBC_*` imports plus
`_ITM_*`, `__gmon_start__` and the `libgcc` `_Unwind_*` family: **0 missing or
undefined non-libc symbols**.

No C static/internal function was skipped: every `static` helper of
`tokenizer.c` (`is_keyword`, `peek_char`, `advance_char`, `skip_whitespace`,
`create_token`, `scan_word`, `scan_number`, `scan_string`, `scan_comment`,
`scan_operator`) and of `analyzer.c` (`track_word`) has a private counterpart in
`src/tokenizer.rs` / `src/analyzer.rs`, and every `static` variable has a field
in `Tokenizer`/`Analyzer` (owned by the process-global singletons of
`src/ffi.rs`).

## Struct-layout parity (checked by `tests/layout.rs`)

| C type | size | align | field offsets |
|--------|------|-------|---------------|
| `token_t` | 280 | 8 | type 0, value 4, length 264, line 272, column 276 |
| `tokenizer_ops_t` | 40 | 8 | 0, 8, 16, 24, 32 |
| `analysis_result_t` | 64 | 8 | 0, 8, 16, 24, 32, 40, 48, 56 |
| `token_type_t` | 4 | 4 | — |

`ffi::CToken`, `ffi::CTokenizerOps` and `ffi::CAnalysisResult` are `#[repr(C)]`
with the same field order and thus the same layout; all three are larger than
16 bytes, so they are returned through the SysV "MEMORY" class hidden-pointer
convention in both languages.

## Feature combinations

`Cargo.toml` declares **no `[features]`** and `c_src/CMakeLists.txt` has no
`option()`/`#ifdef`-driven configuration, so the complete set of build
configurations is:

| # | feature combination | command |
|---|---------------------|---------|
| 1 | (default = empty) | `cargo check --all-targets` |
| 2 | `--no-default-features` (identical to 1) | `cargo check --no-default-features --all-targets` |

Both are verified by `./check_features.sh`, which also re-runs the whole test
suite for each combination.
