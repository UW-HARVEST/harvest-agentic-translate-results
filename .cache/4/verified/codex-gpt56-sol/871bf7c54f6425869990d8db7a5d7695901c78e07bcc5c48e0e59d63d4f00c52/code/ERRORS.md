# Error Surface

This table is derived from every explicit null/argument/EOF rejection branch in
`c_src/src/luggage.c`. The C source has no assertions, enums, explicit numeric
range rejection, allocation-failure handling, or error-return macros.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| E01 | `supersedes` | `directive == NULL` | returns `0` | [x] |
| E02 | `main` | `argc != 5` | writes `Command line error: 4 arguments expected\n` to stderr and exits `1` | [x] |
| E03 | `main` | `scanf("%d ", &time_stamp) == EOF` before any timestamp conversion | stops reading, prints prior matching records, exits `0` | [x] |
| E04 | `main` | `scanf("%8[A-Z0-9] %6[A-Z0-9] ", ...) == EOF` before either ID conversion | drops the incomplete record, prints prior matching records, exits `0` | [x] |
| E05 | `main` | `scanf("%3[A-Z] %3[A-Z]", ...) == EOF` before either airport conversion | drops the incomplete record, prints prior matching records, exits `0` | [x] |
| E06 | `main` | `scanf("%80[^\n]", comments) == EOF` before comment conversion | drops the incomplete record, prints prior matching records, exits `0` | [x] |

An input-matching failure returns `0` from `scanf`, not `EOF`; the C loop does
not reject it and may subsequently read indeterminate arrays. Those cases have
undefined C behavior and are not represented as rejection rows.

The API has no length parameters or enum parameters. Null pointers that the C
source dereferences are undefined behavior rather than explicit C rejections.
The mandatory generic FFI-boundary audit records the platform-observed result:

| # | function | generic boundary input | expected C result | verified |
|---|----------|------------------------|-------------------|----------|
| G01 | `addRoutingDirectiveToList` | null `previous_directive` | terminates with `SIGSEGV` | [x] |
| G02 | `addRoutingDirectiveToList` | null `new_directive` | terminates with `SIGSEGV` | [x] |
| G03 | `supersedes` | nonempty list and null `luggage_id` | terminates with `SIGSEGV` | [x] |
| G04 | `supersedes` | matching luggage and null `departure` | terminates with `SIGSEGV` | [x] |
| G05 | `superseded` | null `directive` | terminates with `SIGSEGV` | [x] |
| G06 | `matches` | null `expected` | terminates with `SIGSEGV` | [x] |
| G07 | `matches` | non-wildcard expected and null `actual` | terminates with `SIGSEGV` | [x] |
| G08 | `printMatchingDirectives` | nonempty list and null first filter | terminates with `SIGSEGV` | [x] |
| G09 | `main` | `argc == 5` and null `argv` | terminates with `SIGSEGV` | [x] |
| G10 | `main` | valid `argv` array with null filter pointers and empty stdin | exits `0`; filters are never dereferenced | [x] |

Safe null forms are also covered: null `supersedes` list, null first directive
for printing, and wildcard `matches` with an unused null `actual`.
