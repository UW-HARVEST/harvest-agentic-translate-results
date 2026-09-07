# Configuration Surface

The public API consists only of `searchAndReplace`. It has no runtime options,
modes, flags, compile-time features, element types, byte-order modes, or numeric
length arguments. The rows below are the pruned cross-product of the branches
and data shapes that `../c_src/src/lib.c` actually distinguishes:

- no match versus at least one match;
- first match at byte zero versus after a copied prefix;
- one match versus later adjacent matches versus later matches separated by a
  copied gap;
- no suffix versus a copied suffix;
- empty versus non-empty replacement;
- one-byte versus multi-byte search text;
- empty versus non-empty original text on the no-match path.

The empty-search shape is non-returning and is therefore in `ERRORS.md`, not the
valid configuration table.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `searchAndReplace` | empty original; non-empty one-byte search; empty replacement; no match (`strdup` path) | [x] |
| 2 | `searchAndReplace` | empty original; non-empty multi-byte search; non-empty replacement; no match (`strdup` path) | [x] |
| 3 | `searchAndReplace` | non-empty original; one-byte search; empty replacement; no match (`strdup` path) | [x] |
| 4 | `searchAndReplace` | non-empty original; multi-byte search; non-empty replacement; no match (`strdup` path) | [x] |
| 5 | `searchAndReplace` | first/only one-byte match at start; no suffix; empty replacement | [x] |
| 6 | `searchAndReplace` | first/only one-byte match at start; no suffix; non-empty replacement | [x] |
| 7 | `searchAndReplace` | first/only multi-byte match at start; suffix present; empty replacement | [x] |
| 8 | `searchAndReplace` | first/only multi-byte match at start; suffix present; non-empty replacement | [x] |
| 9 | `searchAndReplace` | first/only one-byte match after a prefix; no suffix; empty replacement | [x] |
| 10 | `searchAndReplace` | first/only one-byte match after a prefix; no suffix; non-empty replacement | [x] |
| 11 | `searchAndReplace` | first/only multi-byte match after a prefix; suffix present; empty replacement | [x] |
| 12 | `searchAndReplace` | first/only multi-byte match after a prefix; suffix present; non-empty replacement | [x] |
| 13 | `searchAndReplace` | repeated adjacent one-byte matches at start; no suffix; empty replacement | [x] |
| 14 | `searchAndReplace` | repeated adjacent one-byte matches at start; no suffix; non-empty replacement | [x] |
| 15 | `searchAndReplace` | repeated adjacent multi-byte matches after a prefix; suffix present; empty replacement | [x] |
| 16 | `searchAndReplace` | repeated adjacent multi-byte matches after a prefix; suffix present; non-empty replacement | [x] |
| 17 | `searchAndReplace` | repeated gapped one-byte matches at start; no suffix; empty replacement | [x] |
| 18 | `searchAndReplace` | repeated gapped one-byte matches at start; no suffix; non-empty replacement | [x] |
| 19 | `searchAndReplace` | repeated gapped multi-byte matches at start; suffix present; empty replacement | [x] |
| 20 | `searchAndReplace` | repeated gapped multi-byte matches at start; suffix present; non-empty replacement | [x] |
| 21 | `searchAndReplace` | repeated gapped one-byte matches after a prefix; no suffix; empty replacement | [x] |
| 22 | `searchAndReplace` | repeated gapped one-byte matches after a prefix; no suffix; non-empty replacement | [x] |
| 23 | `searchAndReplace` | repeated gapped multi-byte matches after a prefix; suffix present; empty replacement | [x] |
| 24 | `searchAndReplace` | repeated gapped multi-byte matches after a prefix; suffix present; non-empty replacement | [x] |
