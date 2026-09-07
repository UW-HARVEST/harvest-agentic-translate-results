# Error surface

The C source contains no error enums, assertions, explicit range checks,
`RETURN_ERROR`, `return -1`, or `return NULL` paths. Its complete explicit
input-rejection surface consists of these parser fallback branches.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `parse_env_numeric` | `getenv(env_name) == NULL` | Return `default_val`; no warning | [x] |
| 2 | `parse_env_numeric` | Environment value contains `','` (`strchr(env_value, ',') != NULL`) | Print `Warning: Invalid character in <env_name>\n` to stderr and return `default_val` | [x] |
| 3 | `parse_env_numeric` | Environment value contains no comma and contains `';'` (`strchr(env_value, ';') != NULL`) | Print `Warning: Semicolon found in <env_name>\n` to stderr and return `default_val` | [x] |
