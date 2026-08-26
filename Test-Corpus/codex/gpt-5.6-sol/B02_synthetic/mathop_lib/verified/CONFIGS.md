# Configuration Surface

## Build-Time Configurations

Neither `Cargo.toml` nor `c_src/CMakeLists.txt` defines a build-time feature,
option, or conditional source. There is exactly one valid combination:

| # | Cargo features | CMake configuration | [ ] |
|---|----------------|---------------------|-----|
| B1 | `--no-default-features` (empty feature set) | default, PIC enabled | [x] |

## Runtime Configurations

`F` below means an enum representation outside `1..=5`, which takes the
`select_operation` default branch. History shapes are: `N` = NULL inner
history (allocate/reset), `E` = non-NULL with count 0, `M` = count 1..8,
`L` = count 9 (last slot), and `S` = count at least 10 (saturated). Every
randomized arithmetic row includes positive, negative, zero, and integer
boundary-adjacent operands where the C operation is defined.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `is_valid_operation` | each valid character `'1'..='5'` | [x] |
| 2 | `get_operation_priority` | randomized raw enum representations, including `1..=5` and out-of-range values | [x] |
| 3 | `add_operation` | randomized `a`, `b`, and ignored third argument | [x] |
| 4 | `multiply_operation` | randomized `a`, `b`, and ignored third argument | [x] |
| 5 | `subtract_operation` | randomized `a`, `b`, and ignored third argument | [x] |
| 6 | `divide_operation` | randomized operands with `b != 0` and excluding C's `INT_MIN / -1` undefined case | [x] |
| 7 | `modulo_operation` | randomized operands with `b != 0` and excluding C's `INT_MIN % -1` undefined case | [x] |
| 8 | `select_operation` | `op = OP_ADD` | [x] |
| 9 | `select_operation` | `op = OP_MULTIPLY` | [x] |
| 10 | `select_operation` | `op = OP_SUBTRACT` | [x] |
| 11 | `select_operation` | `op = OP_DIVIDE` | [x] |
| 12 | `select_operation` | `op = OP_MODULO` | [x] |
| 13 | `select_operation` | `op = F` (default/add branch) | [x] |
| 14 | `get_computation_timestamp` | current host time shifted right by 29 | [x] |
| 15 | `allocate_results` | count 0 | [x] |
| 16 | `allocate_results` | count 1 | [x] |
| 17 | `allocate_results` | count 2..64; verify every returned byte is zero | [x] |
| 18 | `allocate_results` | negative count (`-1`) | [x] |
| 19 | `allocate_results` | oversized positive count (`INT_MAX`), without dereference | [x] |
| 20 | `perform_computation_with_history` | `op=ADD`, history `N` | [x] |
| 21 | `perform_computation_with_history` | `op=ADD`, history `E` | [x] |
| 22 | `perform_computation_with_history` | `op=ADD`, history `M` | [x] |
| 23 | `perform_computation_with_history` | `op=ADD`, history `L` | [x] |
| 24 | `perform_computation_with_history` | `op=ADD`, history `S` | [x] |
| 25 | `perform_computation_with_history` | `op=MULTIPLY`, history `N` | [x] |
| 26 | `perform_computation_with_history` | `op=MULTIPLY`, history `E` | [x] |
| 27 | `perform_computation_with_history` | `op=MULTIPLY`, history `M` | [x] |
| 28 | `perform_computation_with_history` | `op=MULTIPLY`, history `L` | [x] |
| 29 | `perform_computation_with_history` | `op=MULTIPLY`, history `S` | [x] |
| 30 | `perform_computation_with_history` | `op=SUBTRACT`, history `N` | [x] |
| 31 | `perform_computation_with_history` | `op=SUBTRACT`, history `E` | [x] |
| 32 | `perform_computation_with_history` | `op=SUBTRACT`, history `M` | [x] |
| 33 | `perform_computation_with_history` | `op=SUBTRACT`, history `L` | [x] |
| 34 | `perform_computation_with_history` | `op=SUBTRACT`, history `S` | [x] |
| 35 | `perform_computation_with_history` | `op=DIVIDE`, `b != 0`, history `N` | [x] |
| 36 | `perform_computation_with_history` | `op=DIVIDE`, `b != 0`, history `E` | [x] |
| 37 | `perform_computation_with_history` | `op=DIVIDE`, `b != 0`, history `M` | [x] |
| 38 | `perform_computation_with_history` | `op=DIVIDE`, `b != 0`, history `L` | [x] |
| 39 | `perform_computation_with_history` | `op=DIVIDE`, `b != 0`, history `S` | [x] |
| 40 | `perform_computation_with_history` | `op=MODULO`, `b != 0`, history `N` | [x] |
| 41 | `perform_computation_with_history` | `op=MODULO`, `b != 0`, history `E` | [x] |
| 42 | `perform_computation_with_history` | `op=MODULO`, `b != 0`, history `M` | [x] |
| 43 | `perform_computation_with_history` | `op=MODULO`, `b != 0`, history `L` | [x] |
| 44 | `perform_computation_with_history` | `op=MODULO`, `b != 0`, history `S` | [x] |
| 45 | `perform_computation_with_history` | `op=F` (fallback add), history `N` | [x] |
| 46 | `perform_computation_with_history` | `op=F` (fallback add), history `E` | [x] |
| 47 | `perform_computation_with_history` | `op=F` (fallback add), history `M` | [x] |
| 48 | `perform_computation_with_history` | `op=F` (fallback add), history `L` | [x] |
| 49 | `perform_computation_with_history` | `op=F` (fallback add), history `S` | [x] |

For rows 50-121, `V`/`I` is the valid/invalid `validation_char` branch,
`first` is the operation selected from `param3`, and `second` is the operation
selected from `param4`. Each row is exercised with fresh and saturated static
history and with many bounded randomized operands.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 50 | `mathop` | `V`, first `ADD`, second `ADD` | [x] |
| 51 | `mathop` | `V`, first `ADD`, second `MULTIPLY` | [x] |
| 52 | `mathop` | `V`, first `ADD`, second `SUBTRACT` | [x] |
| 53 | `mathop` | `V`, first `ADD`, second `DIVIDE` | [x] |
| 54 | `mathop` | `V`, first `ADD`, second `MODULO` | [x] |
| 55 | `mathop` | `V`, first `ADD`, second `F` | [x] |
| 56 | `mathop` | `V`, first `MULTIPLY`, second `ADD` | [x] |
| 57 | `mathop` | `V`, first `MULTIPLY`, second `MULTIPLY` | [x] |
| 58 | `mathop` | `V`, first `MULTIPLY`, second `SUBTRACT` | [x] |
| 59 | `mathop` | `V`, first `MULTIPLY`, second `DIVIDE` | [x] |
| 60 | `mathop` | `V`, first `MULTIPLY`, second `MODULO` | [x] |
| 61 | `mathop` | `V`, first `MULTIPLY`, second `F` | [x] |
| 62 | `mathop` | `V`, first `SUBTRACT`, second `ADD` | [x] |
| 63 | `mathop` | `V`, first `SUBTRACT`, second `MULTIPLY` | [x] |
| 64 | `mathop` | `V`, first `SUBTRACT`, second `SUBTRACT` | [x] |
| 65 | `mathop` | `V`, first `SUBTRACT`, second `DIVIDE` | [x] |
| 66 | `mathop` | `V`, first `SUBTRACT`, second `MODULO` | [x] |
| 67 | `mathop` | `V`, first `SUBTRACT`, second `F` | [x] |
| 68 | `mathop` | `V`, first `DIVIDE`, second `ADD` | [x] |
| 69 | `mathop` | `V`, first `DIVIDE`, second `MULTIPLY` | [x] |
| 70 | `mathop` | `V`, first `DIVIDE`, second `SUBTRACT` | [x] |
| 71 | `mathop` | `V`, first `DIVIDE`, second `DIVIDE` | [x] |
| 72 | `mathop` | `V`, first `DIVIDE`, second `MODULO` | [x] |
| 73 | `mathop` | `V`, first `DIVIDE`, second `F` | [x] |
| 74 | `mathop` | `V`, first `MODULO`, second `ADD` | [x] |
| 75 | `mathop` | `V`, first `MODULO`, second `MULTIPLY` | [x] |
| 76 | `mathop` | `V`, first `MODULO`, second `SUBTRACT` | [x] |
| 77 | `mathop` | `V`, first `MODULO`, second `DIVIDE` | [x] |
| 78 | `mathop` | `V`, first `MODULO`, second `MODULO` | [x] |
| 79 | `mathop` | `V`, first `MODULO`, second `F` | [x] |
| 80 | `mathop` | `V`, first `F`, second `ADD` | [x] |
| 81 | `mathop` | `V`, first `F`, second `MULTIPLY` | [x] |
| 82 | `mathop` | `V`, first `F`, second `SUBTRACT` | [x] |
| 83 | `mathop` | `V`, first `F`, second `DIVIDE` | [x] |
| 84 | `mathop` | `V`, first `F`, second `MODULO` | [x] |
| 85 | `mathop` | `V`, first `F`, second `F` | [x] |
| 86 | `mathop` | `I`, first `ADD`, second `ADD` | [x] |
| 87 | `mathop` | `I`, first `ADD`, second `MULTIPLY` | [x] |
| 88 | `mathop` | `I`, first `ADD`, second `SUBTRACT` | [x] |
| 89 | `mathop` | `I`, first `ADD`, second `DIVIDE` | [x] |
| 90 | `mathop` | `I`, first `ADD`, second `MODULO` | [x] |
| 91 | `mathop` | `I`, first `ADD`, second `F` | [x] |
| 92 | `mathop` | `I`, first `MULTIPLY`, second `ADD` | [x] |
| 93 | `mathop` | `I`, first `MULTIPLY`, second `MULTIPLY` | [x] |
| 94 | `mathop` | `I`, first `MULTIPLY`, second `SUBTRACT` | [x] |
| 95 | `mathop` | `I`, first `MULTIPLY`, second `DIVIDE` | [x] |
| 96 | `mathop` | `I`, first `MULTIPLY`, second `MODULO` | [x] |
| 97 | `mathop` | `I`, first `MULTIPLY`, second `F` | [x] |
| 98 | `mathop` | `I`, first `SUBTRACT`, second `ADD` | [x] |
| 99 | `mathop` | `I`, first `SUBTRACT`, second `MULTIPLY` | [x] |
| 100 | `mathop` | `I`, first `SUBTRACT`, second `SUBTRACT` | [x] |
| 101 | `mathop` | `I`, first `SUBTRACT`, second `DIVIDE` | [x] |
| 102 | `mathop` | `I`, first `SUBTRACT`, second `MODULO` | [x] |
| 103 | `mathop` | `I`, first `SUBTRACT`, second `F` | [x] |
| 104 | `mathop` | `I`, first `DIVIDE`, second `ADD` | [x] |
| 105 | `mathop` | `I`, first `DIVIDE`, second `MULTIPLY` | [x] |
| 106 | `mathop` | `I`, first `DIVIDE`, second `SUBTRACT` | [x] |
| 107 | `mathop` | `I`, first `DIVIDE`, second `DIVIDE` | [x] |
| 108 | `mathop` | `I`, first `DIVIDE`, second `MODULO` | [x] |
| 109 | `mathop` | `I`, first `DIVIDE`, second `F` | [x] |
| 110 | `mathop` | `I`, first `MODULO`, second `ADD` | [x] |
| 111 | `mathop` | `I`, first `MODULO`, second `MULTIPLY` | [x] |
| 112 | `mathop` | `I`, first `MODULO`, second `SUBTRACT` | [x] |
| 113 | `mathop` | `I`, first `MODULO`, second `DIVIDE` | [x] |
| 114 | `mathop` | `I`, first `MODULO`, second `MODULO` | [x] |
| 115 | `mathop` | `I`, first `MODULO`, second `F` | [x] |
| 116 | `mathop` | `I`, first `F`, second `ADD` | [x] |
| 117 | `mathop` | `I`, first `F`, second `MULTIPLY` | [x] |
| 118 | `mathop` | `I`, first `F`, second `SUBTRACT` | [x] |
| 119 | `mathop` | `I`, first `F`, second `DIVIDE` | [x] |
| 120 | `mathop` | `I`, first `F`, second `MODULO` | [x] |
| 121 | `mathop` | `I`, first `F`, second `F` | [x] |
