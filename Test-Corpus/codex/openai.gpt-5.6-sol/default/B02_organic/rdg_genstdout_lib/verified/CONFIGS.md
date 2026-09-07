# Configuration Surface

Mechanical source scan:

```text
rg -n 'if\s*\(|switch\s*\(|#if|separator|suffixLen|strlen|strrchr' c_src/include c_src/src
```

Build/runtime axes found in the C source:

- This build selects the Unix `separator = '/'` preprocessor branch. The
  Windows branch is not selected by this CMake build.
- `extractFilename` branches on whether `strrchr()` finds the caller-supplied
  separator. Its public `char` input also admits NUL and high-bit byte values.
- `FIO_createFilename_fromOutDir` branches on path basename shape and on
  whether the byte immediately before the end of `outDirName` is `/`.
- `suffixLen` changes allocation/output-buffer shape even though it does not
  change copied text.
- `Cargo.toml` declares no Cargo features and neither build defines an
  executable target.

For the `FIO_createFilename_fromOutDir` rows, the path-shape axis is:
`E` = empty, `B` = non-empty basename/no slash, `S` = one slash,
`M` = multiple slashes, `T` = trailing slash. The output-directory axis is:
`N` = non-empty/no trailing slash, `R` = non-empty/trailing slash,
`EN` = empty string whose preceding in-allocation byte is not `/`, and
`ER` = empty string whose preceding in-allocation byte is `/`. The latter two
mechanically cover the C expression `outDirName[strlen(outDirName)-1]` for a
pointer to the second byte of a two-byte array. The suffix axis is `0`, `1`,
or `many` bytes.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `extractFilename` | non-empty path; non-NUL separator absent | [x] |
| 2 | `extractFilename` | separator is first byte | [x] |
| 3 | `extractFilename` | exactly one separator in the middle | [x] |
| 4 | `extractFilename` | multiple separators; final occurrence wins | [x] |
| 5 | `extractFilename` | separator is final byte; result points at terminating NUL | [x] |
| 6 | `extractFilename` | empty path; non-NUL separator absent | [x] |
| 7 | `extractFilename` | separator is NUL; `strrchr` finds the terminator and result is one-past it | [x] |
| 8 | `extractFilename` | high-bit separator byte (negative when `char` is signed), present in path | [x] |
| 9 | `FIO_createFilename_fromOutDir` | path `E`; out dir `N`; suffix `0` | [x] |
| 10 | `FIO_createFilename_fromOutDir` | path `E`; out dir `N`; suffix `1` | [x] |
| 11 | `FIO_createFilename_fromOutDir` | path `E`; out dir `N`; suffix `many` | [x] |
| 12 | `FIO_createFilename_fromOutDir` | path `E`; out dir `R`; suffix `0` | [x] |
| 13 | `FIO_createFilename_fromOutDir` | path `E`; out dir `R`; suffix `1` | [x] |
| 14 | `FIO_createFilename_fromOutDir` | path `E`; out dir `R`; suffix `many` | [x] |
| 15 | `FIO_createFilename_fromOutDir` | path `E`; out dir `EN`; suffix `0` | [x] |
| 16 | `FIO_createFilename_fromOutDir` | path `E`; out dir `EN`; suffix `1` | [x] |
| 17 | `FIO_createFilename_fromOutDir` | path `E`; out dir `EN`; suffix `many` | [x] |
| 18 | `FIO_createFilename_fromOutDir` | path `E`; out dir `ER`; suffix `0` | [x] |
| 19 | `FIO_createFilename_fromOutDir` | path `E`; out dir `ER`; suffix `1` | [x] |
| 20 | `FIO_createFilename_fromOutDir` | path `E`; out dir `ER`; suffix `many` | [x] |
| 21 | `FIO_createFilename_fromOutDir` | path `B`; out dir `N`; suffix `0` | [x] |
| 22 | `FIO_createFilename_fromOutDir` | path `B`; out dir `N`; suffix `1` | [x] |
| 23 | `FIO_createFilename_fromOutDir` | path `B`; out dir `N`; suffix `many` | [x] |
| 24 | `FIO_createFilename_fromOutDir` | path `B`; out dir `R`; suffix `0` | [x] |
| 25 | `FIO_createFilename_fromOutDir` | path `B`; out dir `R`; suffix `1` | [x] |
| 26 | `FIO_createFilename_fromOutDir` | path `B`; out dir `R`; suffix `many` | [x] |
| 27 | `FIO_createFilename_fromOutDir` | path `B`; out dir `EN`; suffix `0` | [x] |
| 28 | `FIO_createFilename_fromOutDir` | path `B`; out dir `EN`; suffix `1` | [x] |
| 29 | `FIO_createFilename_fromOutDir` | path `B`; out dir `EN`; suffix `many` | [x] |
| 30 | `FIO_createFilename_fromOutDir` | path `B`; out dir `ER`; suffix `0` | [x] |
| 31 | `FIO_createFilename_fromOutDir` | path `B`; out dir `ER`; suffix `1` | [x] |
| 32 | `FIO_createFilename_fromOutDir` | path `B`; out dir `ER`; suffix `many` | [x] |
| 33 | `FIO_createFilename_fromOutDir` | path `S`; out dir `N`; suffix `0` | [x] |
| 34 | `FIO_createFilename_fromOutDir` | path `S`; out dir `N`; suffix `1` | [x] |
| 35 | `FIO_createFilename_fromOutDir` | path `S`; out dir `N`; suffix `many` | [x] |
| 36 | `FIO_createFilename_fromOutDir` | path `S`; out dir `R`; suffix `0` | [x] |
| 37 | `FIO_createFilename_fromOutDir` | path `S`; out dir `R`; suffix `1` | [x] |
| 38 | `FIO_createFilename_fromOutDir` | path `S`; out dir `R`; suffix `many` | [x] |
| 39 | `FIO_createFilename_fromOutDir` | path `S`; out dir `EN`; suffix `0` | [x] |
| 40 | `FIO_createFilename_fromOutDir` | path `S`; out dir `EN`; suffix `1` | [x] |
| 41 | `FIO_createFilename_fromOutDir` | path `S`; out dir `EN`; suffix `many` | [x] |
| 42 | `FIO_createFilename_fromOutDir` | path `S`; out dir `ER`; suffix `0` | [x] |
| 43 | `FIO_createFilename_fromOutDir` | path `S`; out dir `ER`; suffix `1` | [x] |
| 44 | `FIO_createFilename_fromOutDir` | path `S`; out dir `ER`; suffix `many` | [x] |
| 45 | `FIO_createFilename_fromOutDir` | path `M`; out dir `N`; suffix `0` | [x] |
| 46 | `FIO_createFilename_fromOutDir` | path `M`; out dir `N`; suffix `1` | [x] |
| 47 | `FIO_createFilename_fromOutDir` | path `M`; out dir `N`; suffix `many` | [x] |
| 48 | `FIO_createFilename_fromOutDir` | path `M`; out dir `R`; suffix `0` | [x] |
| 49 | `FIO_createFilename_fromOutDir` | path `M`; out dir `R`; suffix `1` | [x] |
| 50 | `FIO_createFilename_fromOutDir` | path `M`; out dir `R`; suffix `many` | [x] |
| 51 | `FIO_createFilename_fromOutDir` | path `M`; out dir `EN`; suffix `0` | [x] |
| 52 | `FIO_createFilename_fromOutDir` | path `M`; out dir `EN`; suffix `1` | [x] |
| 53 | `FIO_createFilename_fromOutDir` | path `M`; out dir `EN`; suffix `many` | [x] |
| 54 | `FIO_createFilename_fromOutDir` | path `M`; out dir `ER`; suffix `0` | [x] |
| 55 | `FIO_createFilename_fromOutDir` | path `M`; out dir `ER`; suffix `1` | [x] |
| 56 | `FIO_createFilename_fromOutDir` | path `M`; out dir `ER`; suffix `many` | [x] |
| 57 | `FIO_createFilename_fromOutDir` | path `T`; out dir `N`; suffix `0` | [x] |
| 58 | `FIO_createFilename_fromOutDir` | path `T`; out dir `N`; suffix `1` | [x] |
| 59 | `FIO_createFilename_fromOutDir` | path `T`; out dir `N`; suffix `many` | [x] |
| 60 | `FIO_createFilename_fromOutDir` | path `T`; out dir `R`; suffix `0` | [x] |
| 61 | `FIO_createFilename_fromOutDir` | path `T`; out dir `R`; suffix `1` | [x] |
| 62 | `FIO_createFilename_fromOutDir` | path `T`; out dir `R`; suffix `many` | [x] |
| 63 | `FIO_createFilename_fromOutDir` | path `T`; out dir `EN`; suffix `0` | [x] |
| 64 | `FIO_createFilename_fromOutDir` | path `T`; out dir `EN`; suffix `1` | [x] |
| 65 | `FIO_createFilename_fromOutDir` | path `T`; out dir `EN`; suffix `many` | [x] |
| 66 | `FIO_createFilename_fromOutDir` | path `T`; out dir `ER`; suffix `0` | [x] |
| 67 | `FIO_createFilename_fromOutDir` | path `T`; out dir `ER`; suffix `1` | [x] |
| 68 | `FIO_createFilename_fromOutDir` | path `T`; out dir `ER`; suffix `many` | [x] |
