# Error Surface

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `parse_env_numeric` | `getenv(env_name) == NULL` (requested variable is absent) | [x] returns `default_val` |
| 2 | `parse_env_numeric` | value contains `,` | [x] warns `Invalid character` and returns `default_val` |
| 3 | `parse_env_numeric` | value contains `;` and no earlier comma branch applies | [x] warns `Semicolon found` and returns `default_val` |
| 4 | `envy` | computed result is `< 0` after bit operations and base offset | [x] restores the backup and returns original `param1` |
| 5 | `parse_env_numeric` | `env_name == NULL` | [x] no C guard; process terminates on invalid libc input |
| 6 | `init_config_from_env` | `flags == NULL` | [x] no C guard; process terminates on invalid dereference |
| 7 | `perform_operation` | `flags == NULL` | [x] no C guard; process terminates on invalid dereference |
| 8 | `apply_bit_operations` | `flags == NULL` | [x] no C guard; process terminates on invalid dereference |

There are no length parameters, public enums, assertions, error enums, explicit min/max constants, or `return -1`/`RETURN_ERROR` branches. The 3-bit `log_level` boundary and arbitrary raw flag words are valid configuration inputs and are covered in `CONFIGS.md` rows 11-18.
