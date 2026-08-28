# Configuration Surface

The C build is the Unix branch, so `/` is the path separator. There are no
Cargo features and no runtime mode flags. Positive suffix lengths are
randomized over multiple values because they change allocation size even
though they do not change the visible C string.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|--------|
| 1 | `extractFilename` | empty path; non-NUL separator absent | [x] |
| 2 | `extractFilename` | nonempty path; separator absent | [x] |
| 3 | `extractFilename` | separator occurs once before a nonempty basename | [x] |
| 4 | `extractFilename` | separator occurs multiple times; select the last occurrence | [x] |
| 5 | `extractFilename` | separator is the final byte; return the string terminator | [x] |
| 6 | `extractFilename` | separator is NUL; return one byte past the string terminator | [x] |
| 7 | `FIO_createFilename_fromOutDir` | empty path; output directory ends in `/`; `suffixLen == 0` | [x] |
| 8 | `FIO_createFilename_fromOutDir` | empty path; output directory ends in `/`; positive `suffixLen` | [x] |
| 9 | `FIO_createFilename_fromOutDir` | empty path; output directory does not end in `/`; `suffixLen == 0` | [x] |
| 10 | `FIO_createFilename_fromOutDir` | empty path; output directory does not end in `/`; positive `suffixLen` | [x] |
| 11 | `FIO_createFilename_fromOutDir` | path has no `/`; output directory ends in `/`; `suffixLen == 0` | [x] |
| 12 | `FIO_createFilename_fromOutDir` | path has no `/`; output directory ends in `/`; positive `suffixLen` | [x] |
| 13 | `FIO_createFilename_fromOutDir` | path has no `/`; output directory does not end in `/`; `suffixLen == 0` | [x] |
| 14 | `FIO_createFilename_fromOutDir` | path has no `/`; output directory does not end in `/`; positive `suffixLen` | [x] |
| 15 | `FIO_createFilename_fromOutDir` | path has one internal `/`; output directory ends in `/`; `suffixLen == 0` | [x] |
| 16 | `FIO_createFilename_fromOutDir` | path has one internal `/`; output directory ends in `/`; positive `suffixLen` | [x] |
| 17 | `FIO_createFilename_fromOutDir` | path has one internal `/`; output directory does not end in `/`; `suffixLen == 0` | [x] |
| 18 | `FIO_createFilename_fromOutDir` | path has one internal `/`; output directory does not end in `/`; positive `suffixLen` | [x] |
| 19 | `FIO_createFilename_fromOutDir` | path has multiple `/`; output directory ends in `/`; `suffixLen == 0` | [x] |
| 20 | `FIO_createFilename_fromOutDir` | path has multiple `/`; output directory ends in `/`; positive `suffixLen` | [x] |
| 21 | `FIO_createFilename_fromOutDir` | path has multiple `/`; output directory does not end in `/`; `suffixLen == 0` | [x] |
| 22 | `FIO_createFilename_fromOutDir` | path has multiple `/`; output directory does not end in `/`; positive `suffixLen` | [x] |
| 23 | `FIO_createFilename_fromOutDir` | path ends in `/`; output directory ends in `/`; `suffixLen == 0` | [x] |
| 24 | `FIO_createFilename_fromOutDir` | path ends in `/`; output directory ends in `/`; positive `suffixLen` | [x] |
| 25 | `FIO_createFilename_fromOutDir` | path ends in `/`; output directory does not end in `/`; `suffixLen == 0` | [x] |
| 26 | `FIO_createFilename_fromOutDir` | path ends in `/`; output directory does not end in `/`; positive `suffixLen` | [x] |

The public header declares `FIO_createFilename_fromOutDir`; `extractFilename`
is also a public dynamic symbol and is included as the lowest-level entry
point. The only compile-time C configuration is Unix versus Windows separator
handling; this Linux shared library exercises the Unix branch matching the
Rust target.
