# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table, optional dependencies, or implicit
dependency features. `c_src/CMakeLists.txt` has no options, compile
definitions, conditional sources, or backend selection. There is exactly one
valid build-time combination:

| # | Cargo invocation feature set | CMake configuration |
|---|------------------------------|---------------------|
| B1 | `--no-default-features` (no features enabled) | default |

## Runtime and input-shape axes

The complete exported C surface is `printIntPtrLine`, `bad`, `good`, and
`driver`. The source has one runtime branch:

```c
if (useGood) {
    good();
} else {
    bad();
}
```

Thus `useGood == 0` and `useGood != 0` are distinct configurations. There are
no option structs, modes, flags, element types, counts, formats, byte-order
choices, widths, or empty/one/many input shapes. `printIntPtrLine` accepts one
`int`, whose full value domain is exercised even though C does not branch on
the value. `bad` has no caller input and reads an uninitialized local pointer;
its process-level observable result must still be compared because it is an
exported entry point.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| C1 | `printIntPtrLine` | valid pointer to randomized `int`, including `INT_MIN`, `-1`, `0`, `1`, and `INT_MAX` | [x] |
| C2 | `good` | no arguments; direct low-level call | [x] |
| C3 | `bad` | no arguments; direct low-level call with C's uninitialized local pointer state | [x] |
| C4 | `driver` -> `bad` | `useGood == 0` | [x] |
| C5 | `driver` -> `good` | randomized `useGood != 0`, including negative and positive extremes | [x] |

## Completion

- [x] Every runtime/configuration row passes randomized differential testing.
- [x] Every row passes for build-time combination B1.
