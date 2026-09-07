# Error-surface table

The differential rows below are the rejection conditions reachable through
the exported API. They were derived from every conditional rejection in
`c_src/src/lib.c`; the non-public, unreachable guards are accounted for in the
audit appendix.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 [x] | `dataentry` mode 1 / `create_entries` | `param1 > 0`, but allocation of `param1 * sizeof(DataEntry)` returns `NULL` | `-1` |
| 2 [x] | `dataentry` mode 1 / `find_entry` | effective count is valid but no entry has ID `100 + param2` (`param2 < 0` or `param2 >= count`) | `-2` |
| 3 [x] | `dataentry` mode 2 / `create_entries` | `param1 > 0`, but allocation of `param1 * sizeof(DataEntry)` returns `NULL` | `-1` |
| 4 [x] | `dataentry` mode 3 | `param1 < 0` | `0` |
| 5 [x] | `dataentry` mode 3 | `param1 >= 4` | `0` |
| 6 [x] | `dataentry` mode 3 | `0 <= param1 < 4` and `param2 < 0` | `0` |
| 7 [x] | `dataentry` mode 3 | `0 <= param1 < 4` and `param2 >= 3` | `0` |

## Mechanical guard audit

These checks exist in the C source but no exported call can construct their
conditions. They are listed so the source grep has no blind spots; they are
not separate FFI-test rows because both shared libraries expose only
`dataentry`.

| source function | exact check / return | why no public input reaches it |
|-----------------|----------------------|--------------------------------|
| `find_entry` | scan reaches `end`; return `NULL` | Reachable only as row 2, where `dataentry` converts it to `-2`. |
| `process_name` | `dest == NULL`; return `-1` | The sole caller passes local `buffer`. |
| `process_name` | `*dest == '\0'`; return `-1` | The caller first copies `"Default"` into `buffer`. |
| `calculate_lookup` | selected table value is zero; return `0` | All 12 table entries are nonzero and the caller bounds-checks indices. |
| `create_entries` | `count <= 0`; return `NULL` | Modes 1 and 2 replace every nonpositive count with 5 or 3. |
| `modify_entries` | `entries == NULL`; return `-1` | It is called only after successful `create_entries`. |
| `dataentry` mode 1 | `count == 0`; set `result = -1` | Effective count is either positive `param1` or fallback 5. |
| `dataentry` mode 1 | `found->id == 0`; set `result = -2` | Created IDs begin at 100 and increase for positive count without defined signed overflow. |

The API has no pointer, length, or enum parameters. Generic null-pointer and
invalid-enum FFI cases therefore do not exist. Zero, negative, oversized, and
integer-boundary scalar values are covered by rows above and by the valid
configuration tests.
