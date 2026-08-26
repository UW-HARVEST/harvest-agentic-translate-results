# Error Surface

Mechanical source scan:

```text
rg -n 'RETURN_ERROR|return -1|return NULL|assert|NULL|MIN|MAX|if|switch|scanf' c_src
```

The C source has no error-return macro, `return -1`, `return NULL`, assertion,
enum validation, explicit range check, or null check. It does have the
following invalid-input and failed-input paths. Because the pointer operations
have undefined behavior in ISO C, the expected results below describe the
actual default x86-64 build used as the ground truth.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `printIntPtrLine` | `intNumber == NULL`; line 28 dereferences it without a null check | process terminates with `SIGSEGV` before producing output | [x] |
| 2 | `bad` | every call; local pointer `data` is indeterminate and is dereferenced by `printIntPtrLine` | undefined by ISO C; repeated isolated calls compare the exact measured exit/signal and output bytes | [x] |
| 3 | `main` | `scanf("%d", &x)` returns `0` for a non-integer token, leaving initialized `x == 0` | enters `bad`; with both libraries loaded `RTLD_NOW`, process terminates with `SIGSEGV` and no output | [x] |
| 4 | `main` | `scanf("%d", &x)` returns `EOF` before a conversion, leaving initialized `x == 0` | enters `bad`; with both libraries loaded `RTLD_NOW`, process terminates with `SIGSEGV` and no output | [x] |
