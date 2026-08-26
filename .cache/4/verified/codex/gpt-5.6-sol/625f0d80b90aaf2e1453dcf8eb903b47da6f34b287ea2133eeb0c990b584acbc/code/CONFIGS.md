# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table. `c_src/CMakeLists.txt` declares no
options, feature macros, or conditional source files. The complete valid
feature set is therefore:

| # | Cargo invocation suffix | C configuration |
|---|-------------------------|-----------------|
| 1 | `--no-default-features` (empty feature set) | Default and only CMake configuration |

## Runtime Axes

Mechanical C branches and operations:

```text
main: argc != 2
main: strtol(..., 10), then end == argv[1]
main: ten iterations, i = 0 through 9
main: static_sum(i * stride)
static_sum: process-lifetime static int state, sum += update
```

There are no public headers, option setters, modes, flags, element types,
formats, byte-order choices, or configurable counts. The complete public API
is the two symbols reported in `SYMBOLS.md`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `static_sum` | Direct low-level calls: first call and many calls; zero, positive, negative, and boundary `int` updates; cumulative process-lifetime state | [x] |
| 2 | `main`, `static_sum` | `argc == 2`; canonical unsigned decimal text; zero and positive strides; ten output iterations | [x] |
| 3 | `main`, `static_sum` | `argc == 2`; leading C whitespace and optional `+`/`-`; negative and positive strides | [x] |
| 4 | `main`, `static_sum` | `argc == 2`; valid decimal prefix followed by nonnumeric suffix, which `strtol` accepts because at least one character is consumed | [x] |
| 5 | `main`, `static_sum` | `argc == 2`; values at/beyond C `long` and `int` boundaries; `strtol` saturation/`long`-to-`int` truncation; overflowing `int` arithmetic as produced by the built C artifact | [x] |
| 6 | `static_sum`, `main` | Mixed direct and repeated top-level calls in one loaded library; static state carries across entry points and invocations | [x] |
