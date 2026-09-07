# Configuration surface

Legend for `envy` rows:

- `V`, `D`, `O`: resulting verbose/debug/optimize flag is on; `v`, `d`, `o`: off.
- `3Z`/`3N`: `param3` is zero/nonzero. `4Z`/`4N`: `param4` is zero/nonzero.
- `R+`/`R-`: result immediately before `if (result < 0)` is nonnegative/negative.
- Every `envy` row is exercised with all four
  `(PROG_BASE_OFFSET unset/set) × (PROG_MULTIPLIER unset/set)` combinations.
  This covers the parser-source branches inside the composed operation without
  duplicating each end-to-end branch row four times.
- Randomized scalar inputs include zero, positive, negative, odd/even, and
  near-boundary values where the corresponding C operation is defined.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `parse_env_numeric` | Named environment variable unset; randomized default | [x] |
| 2 | `parse_env_numeric` | Named environment variable set to a valid `atoi` string: zero/positive/negative, whitespace/sign, numeric prefix/trailing text | [x] |
| 3 | `init_config_from_env` | verbose absent; debug absent; optimize absent | [x] |
| 4 | `init_config_from_env` | verbose absent; debug absent; optimize present | [x] |
| 5 | `init_config_from_env` | verbose absent; debug present without `1`; optimize absent | [x] |
| 6 | `init_config_from_env` | verbose absent; debug present without `1`; optimize present | [x] |
| 7 | `init_config_from_env` | verbose absent; debug present containing `1`; optimize absent | [x] |
| 8 | `init_config_from_env` | verbose absent; debug present containing `1`; optimize present | [x] |
| 9 | `init_config_from_env` | verbose present without `1`; debug absent; optimize absent | [x] |
| 10 | `init_config_from_env` | verbose present without `1`; debug absent; optimize present | [x] |
| 11 | `init_config_from_env` | verbose present without `1`; debug present without `1`; optimize absent | [x] |
| 12 | `init_config_from_env` | verbose present without `1`; debug present without `1`; optimize present | [x] |
| 13 | `init_config_from_env` | verbose present without `1`; debug present containing `1`; optimize absent | [x] |
| 14 | `init_config_from_env` | verbose present without `1`; debug present containing `1`; optimize present | [x] |
| 15 | `init_config_from_env` | verbose present containing `1`; debug absent; optimize absent | [x] |
| 16 | `init_config_from_env` | verbose present containing `1`; debug absent; optimize present | [x] |
| 17 | `init_config_from_env` | verbose present containing `1`; debug present without `1`; optimize absent | [x] |
| 18 | `init_config_from_env` | verbose present containing `1`; debug present without `1`; optimize present | [x] |
| 19 | `init_config_from_env` | verbose present containing `1`; debug present containing `1`; optimize absent | [x] |
| 20 | `init_config_from_env` | verbose present containing `1`; debug present containing `1`; optimize present | [x] |
| 21 | `perform_operation` | optimize off; debug off; log level 0..7; randomized val1 and odd/even val2 signs | [x] |
| 22 | `perform_operation` | optimize off; debug on; log level 0..7; randomized val1 and odd/even val2 signs | [x] |
| 23 | `perform_operation` | optimize on; debug off; log level 0..7 ignored; randomized val1/val2 signs | [x] |
| 24 | `perform_operation` | optimize on; debug on; log level 0..7 ignored; randomized val1/val2 signs | [x] |
| 25 | `apply_bit_operations` | verbose off; cache off; randomized negative/zero/positive value | [x] |
| 26 | `apply_bit_operations` | verbose off; cache on; randomized negative/zero/positive value | [x] |
| 27 | `apply_bit_operations` | verbose on; cache off; randomized safely shiftable negative/zero/positive value | [x] |
| 28 | `apply_bit_operations` | verbose on; cache on; randomized safely shiftable negative/zero/positive value | [x] |
| 29 | `envy` | `v d o 3Z 4Z R+` | [x] |
| 30 | `envy` | `v d o 3Z 4Z R-` | [x] |
| 31 | `envy` | `v d o 3Z 4N R+` | [x] |
| 32 | `envy` | `v d o 3Z 4N R-` | [x] |
| 33 | `envy` | `v d o 3N 4Z R+` | [x] |
| 34 | `envy` | `v d o 3N 4Z R-` | [x] |
| 35 | `envy` | `v d o 3N 4N R+` | [x] |
| 36 | `envy` | `v d o 3N 4N R-` | [x] |
| 37 | `envy` | `v d O 3Z 4Z R+` | [x] |
| 38 | `envy` | `v d O 3Z 4Z R-` | [x] |
| 39 | `envy` | `v d O 3Z 4N R+` | [x] |
| 40 | `envy` | `v d O 3Z 4N R-` | [x] |
| 41 | `envy` | `v d O 3N 4Z R+` | [x] |
| 42 | `envy` | `v d O 3N 4Z R-` | [x] |
| 43 | `envy` | `v d O 3N 4N R+` | [x] |
| 44 | `envy` | `v d O 3N 4N R-` | [x] |
| 45 | `envy` | `v D o 3Z 4Z R+` | [x] |
| 46 | `envy` | `v D o 3Z 4Z R-` | [x] |
| 47 | `envy` | `v D o 3Z 4N R+` | [x] |
| 48 | `envy` | `v D o 3Z 4N R-` | [x] |
| 49 | `envy` | `v D o 3N 4Z R+` | [x] |
| 50 | `envy` | `v D o 3N 4Z R-` | [x] |
| 51 | `envy` | `v D o 3N 4N R+` | [x] |
| 52 | `envy` | `v D o 3N 4N R-` | [x] |
| 53 | `envy` | `v D O 3Z 4Z R+` | [x] |
| 54 | `envy` | `v D O 3Z 4Z R-` | [x] |
| 55 | `envy` | `v D O 3Z 4N R+` | [x] |
| 56 | `envy` | `v D O 3Z 4N R-` | [x] |
| 57 | `envy` | `v D O 3N 4Z R+` | [x] |
| 58 | `envy` | `v D O 3N 4Z R-` | [x] |
| 59 | `envy` | `v D O 3N 4N R+` | [x] |
| 60 | `envy` | `v D O 3N 4N R-` | [x] |
| 61 | `envy` | `V d o 3Z 4Z R+` | [x] |
| 62 | `envy` | `V d o 3Z 4Z R-` | [x] |
| 63 | `envy` | `V d o 3Z 4N R+` | [x] |
| 64 | `envy` | `V d o 3Z 4N R-` | [x] |
| 65 | `envy` | `V d o 3N 4Z R+` | [x] |
| 66 | `envy` | `V d o 3N 4Z R-` | [x] |
| 67 | `envy` | `V d o 3N 4N R+` | [x] |
| 68 | `envy` | `V d o 3N 4N R-` | [x] |
| 69 | `envy` | `V d O 3Z 4Z R+` | [x] |
| 70 | `envy` | `V d O 3Z 4Z R-` | [x] |
| 71 | `envy` | `V d O 3Z 4N R+` | [x] |
| 72 | `envy` | `V d O 3Z 4N R-` | [x] |
| 73 | `envy` | `V d O 3N 4Z R+` | [x] |
| 74 | `envy` | `V d O 3N 4Z R-` | [x] |
| 75 | `envy` | `V d O 3N 4N R+` | [x] |
| 76 | `envy` | `V d O 3N 4N R-` | [x] |
| 77 | `envy` | `V D o 3Z 4Z R+` | [x] |
| 78 | `envy` | `V D o 3Z 4Z R-` | [x] |
| 79 | `envy` | `V D o 3Z 4N R+` | [x] |
| 80 | `envy` | `V D o 3Z 4N R-` | [x] |
| 81 | `envy` | `V D o 3N 4Z R+` | [x] |
| 82 | `envy` | `V D o 3N 4Z R-` | [x] |
| 83 | `envy` | `V D o 3N 4N R+` | [x] |
| 84 | `envy` | `V D o 3N 4N R-` | [x] |
| 85 | `envy` | `V D O 3Z 4Z R+` | [x] |
| 86 | `envy` | `V D O 3Z 4Z R-` | [x] |
| 87 | `envy` | `V D O 3Z 4N R+` | [x] |
| 88 | `envy` | `V D O 3Z 4N R-` | [x] |
| 89 | `envy` | `V D O 3N 4Z R+` | [x] |
| 90 | `envy` | `V D O 3N 4Z R-` | [x] |
| 91 | `envy` | `V D O 3N 4N R+` | [x] |
| 92 | `envy` | `V D O 3N 4N R-` | [x] |
