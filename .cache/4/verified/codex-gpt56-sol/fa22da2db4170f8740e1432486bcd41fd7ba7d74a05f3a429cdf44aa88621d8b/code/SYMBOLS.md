# Dynamic Symbol Surface

Source library: `c_src/build/libdriver_c.so`, built from the two implementation
files backing the public headers:

```text
cc -shared -fPIC -Iinclude src/analyzer.c src/tokenizer.c \
  -o build/libdriver_c.so
```

Symbols were extracted mechanically with:

```text
nm -D --defined-only c_src/build/libdriver_c.so | awk '{print $3}' | sort
```

| C dynamic symbol | Rust dynamic symbol | Status |
|---|---|---|
| `analyze_text` | `analyze_text` | present |
| `analyzer_init` | `analyzer_init` | present |
| `calculate_complexity_score` | `calculate_complexity_score` | present |
| `find_patterns` | `find_patterns` | present |
| `get_tokenizer_ops` | `get_tokenizer_ops` | present |
| `print_token_distribution` | `print_token_distribution` | present |
| `tokenizer_get_stats` | `tokenizer_get_stats` | present |
| `tokenizer_load_text` | `tokenizer_load_text` | present |
| `tokenizer_next_token` | `tokenizer_next_token` | present |
| `tokenizer_peek_token` | `tokenizer_peek_token` | present |
| `tokenizer_reset` | `tokenizer_reset` | present |

Missing from Rust: **0**.

The C library's strong undefined dynamic references are only C runtime symbols:
`__ctype_b_loc`, `fwrite`, `memset`, `printf`, `puts`, `stderr`, `strchr`,
`strcmp`, `strcpy`, `strlen`, `strncpy`, and `strstr`. Its remaining undefined
entries (`_ITM_*`, `__cxa_finalize`, and `__gmon_start__`) are weak toolchain
runtime hooks.
