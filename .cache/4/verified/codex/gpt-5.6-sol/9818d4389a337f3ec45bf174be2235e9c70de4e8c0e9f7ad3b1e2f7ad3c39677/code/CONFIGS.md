# Configuration surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
options or conditional compilation. There is exactly one valid feature
combination:

| # | Cargo feature set | C configuration | [ ] |
|---|-------------------|-----------------|-----|
| 1 | empty (`--no-default-features --features ""`) | default | [x] |

## Runtime configurations

Mechanically scanned source scope: `c_src/src/**/*.{c,h}`. It contains no
runtime flags, option setters, branches, switches, loops, conditional
compilation, headers, element types, formats, byte-order modes, or
size/count/width inputs. The rows below cover both exported entry points and
the input outcomes induced by the literal `scanf("%d", &x)` operation.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver(int)` | no options; arbitrary by-value C `int`, including zero, signs, and `INT_MIN`/`INT_MAX` boundaries | [x] |
| 2 | `main()` | no options; stdin contains a successfully convertible decimal C `int`, including accepted whitespace/sign syntax | [x] |
| 3 | `main()` | no options; stdin contains a nonnumeric token, so `scanf` reports matching failure and initialized `x == 0` is retained | [x] |
| 4 | `main()` | no options; stdin is empty, so `scanf` reports input failure and initialized `x == 0` is retained | [x] |
