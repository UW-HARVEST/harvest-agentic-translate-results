# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no build
option or conditional source. There is exactly one valid feature combination:

| # | Cargo feature combination | CMake configuration | checked |
|---|---------------------------|---------------------|---------|
| B01 | no features (`--no-default-features`) | default, PIC enabled | [x] |

## Runtime Configurations

Rows were derived from every branch in `c_src/src/lib.c`. Randomized inputs for
the arithmetic rows include zero, positive and negative values, odd/even
division operands, all `log_level` bitfield values, and accepted environment
text forms. Arithmetic inputs stay within C's defined signed-integer range.

For `envy`, `V`, `D`, and `O` mean the `PROG_VERBOSE`, `PROG_DEBUG`, and
`PROG_OPTIMIZE` states. `P3`/`P4` indicate whether those parameters are zero.
`R-` means inputs are chosen so the post-offset result is negative and the
backup-restore branch runs; `R+` means it is nonnegative. Each row randomizes
accepted multiplier sources: absent (octal default `012`) and present with
parseable text. `R+` rows use nonnegative parseable base offsets; `R-` rows use
a negative parseable base offset to reach the recovery branch without signed
overflow. The parser's absent/default and accepted-text branches are isolated
in P01-P05.

| # | entry point(s) | configuration (options set + input shape) | pass |
|---|----------------|--------------------------------------------|------|
| P01 | `parse_env_numeric` | environment variable absent; randomized default | [x] |
| P02 | `parse_env_numeric` | empty or nonnumeric environment text (`atoi` returns zero) | [x] |
| P03 | `parse_env_numeric` | signed decimal text, optional leading whitespace | [x] |
| P04 | `parse_env_numeric` | decimal prefix followed by accepted trailing text | [x] |
| P05 | `parse_env_numeric` | zero, positive, and negative in-range decimal text | [x] |
| I01 | `init_config_from_env` | V=0, D=0, O=0 | [x] |
| I02 | `init_config_from_env` | V=0, D=0, O=1 | [x] |
| I03 | `init_config_from_env` | V=0, D=1, O=0 | [x] |
| I04 | `init_config_from_env` | V=0, D=1, O=1 | [x] |
| I05 | `init_config_from_env` | V=1, D=0, O=0 | [x] |
| I06 | `init_config_from_env` | V=1, D=0, O=1 | [x] |
| I07 | `init_config_from_env` | V=1, D=1, O=0 | [x] |
| I08 | `init_config_from_env` | V=1, D=1, O=1 | [x] |
| O01 | `perform_operation` | optimize=0, debug=0; all log levels and scalar shapes | [x] |
| O02 | `perform_operation` | optimize=0, debug=1; all log levels and scalar shapes | [x] |
| O03 | `perform_operation` | optimize=1, debug=0; all log levels and scalar shapes | [x] |
| O04 | `perform_operation` | optimize=1, debug=1; all log levels and scalar shapes | [x] |
| A01 | `apply_bit_operations` | verbose=0, cache=0; zero/positive/negative values | [x] |
| A02 | `apply_bit_operations` | verbose=0, cache=1; zero/positive/negative values | [x] |
| A03 | `apply_bit_operations` | verbose=1, cache=0; zero/positive safely shiftable values | [x] |
| A04 | `apply_bit_operations` | verbose=1, cache=1; zero/positive safely shiftable values | [x] |
| N001 | `envy` | V=0 D=0 O=0, P3=0 P4=0, R+ | [x] |
| N002 | `envy` | V=0 D=0 O=0, P3=0 P4=0, R- | [x] |
| N003 | `envy` | V=0 D=0 O=0, P3=0 P4=1, R+ | [x] |
| N004 | `envy` | V=0 D=0 O=0, P3=0 P4=1, R- | [x] |
| N005 | `envy` | V=0 D=0 O=0, P3=1 P4=0, R+ | [x] |
| N006 | `envy` | V=0 D=0 O=0, P3=1 P4=0, R- | [x] |
| N007 | `envy` | V=0 D=0 O=0, P3=1 P4=1, R+ | [x] |
| N008 | `envy` | V=0 D=0 O=0, P3=1 P4=1, R- | [x] |
| N009 | `envy` | V=0 D=0 O=1, P3=0 P4=0, R+ | [x] |
| N010 | `envy` | V=0 D=0 O=1, P3=0 P4=0, R- | [x] |
| N011 | `envy` | V=0 D=0 O=1, P3=0 P4=1, R+ | [x] |
| N012 | `envy` | V=0 D=0 O=1, P3=0 P4=1, R- | [x] |
| N013 | `envy` | V=0 D=0 O=1, P3=1 P4=0, R+ | [x] |
| N014 | `envy` | V=0 D=0 O=1, P3=1 P4=0, R- | [x] |
| N015 | `envy` | V=0 D=0 O=1, P3=1 P4=1, R+ | [x] |
| N016 | `envy` | V=0 D=0 O=1, P3=1 P4=1, R- | [x] |
| N017 | `envy` | V=0 D=1 O=0, P3=0 P4=0, R+ | [x] |
| N018 | `envy` | V=0 D=1 O=0, P3=0 P4=0, R- | [x] |
| N019 | `envy` | V=0 D=1 O=0, P3=0 P4=1, R+ | [x] |
| N020 | `envy` | V=0 D=1 O=0, P3=0 P4=1, R- | [x] |
| N021 | `envy` | V=0 D=1 O=0, P3=1 P4=0, R+ | [x] |
| N022 | `envy` | V=0 D=1 O=0, P3=1 P4=0, R- | [x] |
| N023 | `envy` | V=0 D=1 O=0, P3=1 P4=1, R+ | [x] |
| N024 | `envy` | V=0 D=1 O=0, P3=1 P4=1, R- | [x] |
| N025 | `envy` | V=0 D=1 O=1, P3=0 P4=0, R+ | [x] |
| N026 | `envy` | V=0 D=1 O=1, P3=0 P4=0, R- | [x] |
| N027 | `envy` | V=0 D=1 O=1, P3=0 P4=1, R+ | [x] |
| N028 | `envy` | V=0 D=1 O=1, P3=0 P4=1, R- | [x] |
| N029 | `envy` | V=0 D=1 O=1, P3=1 P4=0, R+ | [x] |
| N030 | `envy` | V=0 D=1 O=1, P3=1 P4=0, R- | [x] |
| N031 | `envy` | V=0 D=1 O=1, P3=1 P4=1, R+ | [x] |
| N032 | `envy` | V=0 D=1 O=1, P3=1 P4=1, R- | [x] |
| N033 | `envy` | V=1 D=0 O=0, P3=0 P4=0, R+ | [x] |
| N034 | `envy` | V=1 D=0 O=0, P3=0 P4=0, R- | [x] |
| N035 | `envy` | V=1 D=0 O=0, P3=0 P4=1, R+ | [x] |
| N036 | `envy` | V=1 D=0 O=0, P3=0 P4=1, R- | [x] |
| N037 | `envy` | V=1 D=0 O=0, P3=1 P4=0, R+ | [x] |
| N038 | `envy` | V=1 D=0 O=0, P3=1 P4=0, R- | [x] |
| N039 | `envy` | V=1 D=0 O=0, P3=1 P4=1, R+ | [x] |
| N040 | `envy` | V=1 D=0 O=0, P3=1 P4=1, R- | [x] |
| N041 | `envy` | V=1 D=0 O=1, P3=0 P4=0, R+ | [x] |
| N042 | `envy` | V=1 D=0 O=1, P3=0 P4=0, R- | [x] |
| N043 | `envy` | V=1 D=0 O=1, P3=0 P4=1, R+ | [x] |
| N044 | `envy` | V=1 D=0 O=1, P3=0 P4=1, R- | [x] |
| N045 | `envy` | V=1 D=0 O=1, P3=1 P4=0, R+ | [x] |
| N046 | `envy` | V=1 D=0 O=1, P3=1 P4=0, R- | [x] |
| N047 | `envy` | V=1 D=0 O=1, P3=1 P4=1, R+ | [x] |
| N048 | `envy` | V=1 D=0 O=1, P3=1 P4=1, R- | [x] |
| N049 | `envy` | V=1 D=1 O=0, P3=0 P4=0, R+ | [x] |
| N050 | `envy` | V=1 D=1 O=0, P3=0 P4=0, R- | [x] |
| N051 | `envy` | V=1 D=1 O=0, P3=0 P4=1, R+ | [x] |
| N052 | `envy` | V=1 D=1 O=0, P3=0 P4=1, R- | [x] |
| N053 | `envy` | V=1 D=1 O=0, P3=1 P4=0, R+ | [x] |
| N054 | `envy` | V=1 D=1 O=0, P3=1 P4=0, R- | [x] |
| N055 | `envy` | V=1 D=1 O=0, P3=1 P4=1, R+ | [x] |
| N056 | `envy` | V=1 D=1 O=0, P3=1 P4=1, R- | [x] |
| N057 | `envy` | V=1 D=1 O=1, P3=0 P4=0, R+ | [x] |
| N058 | `envy` | V=1 D=1 O=1, P3=0 P4=0, R- | [x] |
| N059 | `envy` | V=1 D=1 O=1, P3=0 P4=1, R+ | [x] |
| N060 | `envy` | V=1 D=1 O=1, P3=0 P4=1, R- | [x] |
| N061 | `envy` | V=1 D=1 O=1, P3=1 P4=0, R+ | [x] |
| N062 | `envy` | V=1 D=1 O=1, P3=1 P4=0, R- | [x] |
| N063 | `envy` | V=1 D=1 O=1, P3=1 P4=1, R+ | [x] |
| N064 | `envy` | V=1 D=1 O=1, P3=1 P4=1, R- | [x] |
