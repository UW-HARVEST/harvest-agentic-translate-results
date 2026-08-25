# Error Surface

The C source was mechanically scanned with:

```text
rg -n 'RETURN_ERROR|return[[:space:]]+(-1|NULL)|assert[[:space:]]*\(|if[[:space:]]*\(|switch[[:space:]]*\(|#[[:space:]]*(if|ifdef|ifndef)|NULL|MIN|MAX|enum' c_src --glob '*.{c,h}'
rg -n '\breturn\b' c_src --glob '*.{c,h}'
```

The only match from the rejection scan is the header include guard. The only
returns are `return 0` in `helloworld` and `return helloworld()` in `main`.
There are no rejection branches, error sentinels, assertions, checks, enums,
parameters, pointers, or lengths.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are zero error-surface rows to test. Generic null, zero/oversized length,
range, and invalid-enum cases are not applicable because neither exported
function accepts an argument.
