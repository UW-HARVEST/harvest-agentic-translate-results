# Error Surface

Mechanical source scan:

```text
$ rg -n 'return|assert|if|switch|#if|#ifdef|NULL|MIN|MAX|enum' c_src
c_src/src/main.c:40:  if (argc != 2) {
c_src/src/main.c:42:    return 1;
c_src/src/main.c:47:  if (end == argv[1]) {
c_src/src/main.c:50:    return 1;
c_src/src/main.c:57:  return 0;
```

There are no assertions, explicit pointer checks, length checks, enums, or
min/max constants in the C source.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `main` | `argc != 2` | Print `Error: should only be a single (integer) argument!\n`; return `1` without reading `argv` |
| 2 | `main` | `argc == 2` and `strtol(argv[1], &end, 10)` leaves `end == argv[1]` because it converts no characters | Print `Error: first argument must be an integer!\n`; return `1` |

Phase C status:

- [x] Row 1
- [x] Row 2
- [x] Generic null-pointer boundary: null `argv` with `argc != 2`
- [x] Generic null-pointer boundary: null `argv` or null `argv[1]` with `argc == 2`
- [x] Zero/oversized lengths: not applicable; the API has no length argument
- [x] Out-of-range enum values: not applicable; the API has no enum argument
