# Configuration surface

Mechanically derived from the exported functions and the `if`, `else if`, loop,
bit-mask, modulo, and pointer-comparison branches in `../c_src/src/lib.c`.
There are no compile-time or Cargo features and no runtime mode-setting API.
All rows therefore apply to the sole/default feature configuration.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `create_block` | empty NUL-terminated name; randomized `id` and all `uint8_t` flag values | [x] |
| C2 | `create_block` | name length 1 through 30; randomized bytes without interior NUL, `id`, and flags | [x] |
| C3 | `create_block` | exactly 31 name bytes plus terminator (fills `name[32]`); randomized `id` and flags | [x] |
| C4 | `allocate_block`, `free_block` | count 0; randomized safe signed `init_value`; inspect size/data then free | [x] |
| C5 | `allocate_block`, `free_block` | count 1; randomized safe signed `init_value`; inspect sole element then free | [x] |
| C6 | `allocate_block`, `free_block` | count 2 through 256; randomized safe signed `init_value`; inspect every element then free | [x] |
| C7 | `free_block` | null block pointer | [x] |
| C8 | `free_block` | allocated block whose `data` field is set to null after its data allocation is separately freed | [x] |
| C9 | `compute_hash` | data pointer `<` and block pointer `<`; expected branch contributions 100 + 10 | [x] |
| C10 | `compute_hash` | data pointer `<` and block pointer `>`; expected branch contributions 100 + 20 | [x] |
| C11 | `compute_hash` | data pointer `>` and block pointer `<`; expected branch contributions 200 + 10 | [x] |
| C12 | `compute_hash` | data pointer `>` and block pointer `>`; expected branch contributions 200 + 20 | [x] |
| C13 | `compute_hash` | equal data pointers in distinct blocks with block pointer `<`; expected 0 + 10 | [x] |
| C14 | `compute_hash` | equal data pointers in distinct blocks with block pointer `>`; expected 0 + 20 | [x] |
| C15 | `compute_hash` | identical block pointer (therefore identical data pointer); expected 0 | [x] |
| C16 | `betagamma` | `param1 % 10 == -5`, producing internal count 0; randomized bounded `param2..4` exercising all four fixed flag-mask contributions | [x] |
| C17 | `betagamma` | `param1 % 10 == -4`, producing internal count 1; randomized bounded `param2..4` exercising all four fixed flag-mask contributions | [x] |
| C18 | `betagamma` | `param1 % 10` from `-3` through `9`, producing counts 2 through 14; randomized bounded parameters and every remainder/count | [x] |
| C19 | `betagamma` | defined arithmetic boundary cases near signed limits, selected so every C addition/multiplication remains representable | [x] |

Feature combinations: **default only** (the `[features]` table is absent).
