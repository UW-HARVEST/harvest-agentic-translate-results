# Configuration-Surface Table

## Build-Time Configurations

Neither `Cargo.toml` nor `c_src/CMakeLists.txt` declares a feature, backend,
preprocessor option, or optional source. The complete valid feature set is:

| # | Cargo invocation feature set | C configuration | verified |
|---|------------------------------|-----------------|-----|
| B1 | `--no-default-features` (empty set) | Default CMake configuration with position-independent code | [x] |

## Runtime Configurations

Rows are derived from the branches at C lines 30, 45, 52, 59, and 66, plus
the pointer-identity shape accepted by the lowest-level public function. Error
branches at lines 45, 52, and 59 are tracked in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|-----|
| 1 | `static_alias` | `outer` points to caller storage and `*outer < inner`; caller value is incremented by `inner`, and the returned pointer aliases `outer` | [x] |
| 2 | `static_alias` | `outer` points to caller storage and `*outer == inner`; `inner` is incremented by `*outer`, and the returned pointer aliases static `inner` | [x] |
| 3 | `static_alias` | `outer` points to caller storage and `*outer > inner`; `inner` is incremented by `*outer`, and the returned pointer aliases static `inner` | [x] |
| 4 | `static_alias` | `outer` already aliases static `inner`; equality is necessarily true, `inner` doubles, and the returned pointer preserves that alias | [x] |
| 5 | `static_alias` | Many calls following each returned pointer, with values that keep caller storage active, transition to static storage, and then repeatedly alias static storage | [x] |
| 6 | `main`, `static_alias` | Valid decimal initial value and negative `iterations`; loop body is empty | [x] |
| 7 | `main`, `static_alias` | Valid decimal initial value and zero `iterations`; loop body is empty at the zero boundary | [x] |
| 8 | `main`, `static_alias` | One iteration with initial value `< 1`; output comes from caller storage | [x] |
| 9 | `main`, `static_alias` | One iteration with initial value `== 1`; output comes from static storage via the equality boundary | [x] |
| 10 | `main`, `static_alias` | One iteration with initial value `> 1`; output comes from static storage | [x] |
| 11 | `main`, `static_alias` | Many iterations with initial value `< 1`, ending before the returned pointer transitions from caller to static storage | [x] |
| 12 | `main`, `static_alias` | Many iterations with initial value `< 1`, with the caller-to-static pointer transition on the final iteration | [x] |
| 13 | `main`, `static_alias` | Many iterations with initial value `< 1`, with the transition before the final iteration and subsequent static aliasing | [x] |
| 14 | `main`, `static_alias` | Many iterations with initial value `== 1`; first call selects static storage and later calls double it | [x] |
| 15 | `main`, `static_alias` | Many iterations with initial value `> 1`; first call selects static storage and later calls double it | [x] |
| 16 | `main` | Both numbers parse with leading ASCII whitespace and explicit signs | [x] |
| 17 | `main` | Both numbers have a valid decimal prefix followed by nonnumeric bytes; trailing bytes are accepted because C checks only `end == argv[n]` | [x] |
| 18 | `main` | A decimal argument overflows C `long`; `strtol` saturates, C ignores `errno`, and the result is converted to `int` | [x] |
| 19 | `main` | `argc == 3`, `argv[0] == NULL`, and both numeric argument pointers are valid; unused program-name pointer does not affect execution | [x] |

There are no runtime flags, modes, byte-order options, formats, element types,
length-bearing buffers, or public entry points beyond `main` and
`static_alias`.
