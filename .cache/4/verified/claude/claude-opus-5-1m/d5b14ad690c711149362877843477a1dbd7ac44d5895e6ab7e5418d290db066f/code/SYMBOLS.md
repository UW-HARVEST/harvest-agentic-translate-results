# SYMBOLS.md — surface map (Phase A)

## 0. What kind of artifact is this?

`c_src/CMakeLists.txt` contains exactly one target:

```cmake
add_executable(driver src/main.c)
```

There is **no shared library** and there are **no public headers** — `c_src/`
contains a single translation unit (`src/main.c`, 685 lines) whose every
function has external linkage but is never exported dynamically, because the
artifact is a program, not a library. `Cargo.toml` mirrors this with a single
`[[bin]] name = "driver"` target.

Consequences for this verification task:

* The "`.so` loaded through `libloading`" comparison cannot apply literally —
  neither side produces a `.so`, and neither side exports any callable
  function. The ABI-level surface of both artifacts is the **process
  boundary**: `argv`/`stdin` in, `stdout`/`stderr`/exit-status out.
* Therefore the differential tests in `tests/` spawn **both real executables**
  (the CMake-built C binary and the Cargo-built Rust binary) and compare the
  raw bytes of `stdout`, `stderr` and the exit status / terminating signal.
  Nothing is called in-process; both sides are driven exactly as an external
  consumer drives them, which also covers `main`'s own start-up/teardown
  behaviour (start-up banner, stdio buffering, `exit()` flushing, death by
  signal). `libloading` is present in `[dev-dependencies]` as required by the
  task description.

## 1. `nm -D` comparison (dynamic symbol tables)

```
$ nm -D --defined-only c_src/build/driver
0000000000407080 B stdin@GLIBC_2.2.5

$ nm -D --defined-only target/release/driver
(no output)
```

The C binary's dynamic symbol table contains exactly **one** defined symbol,
`stdin@GLIBC_2.2.5`. That symbol is **not implemented by `main.c`**: it is a
glibc *copy relocation* that the static linker synthesises because `main.c`
mentions the `stdin` object in `fgets(input, MAX_INPUT, stdin)`. It is a data
copy of libc's own `FILE *stdin`, so it is neither a translatable function nor
part of the program's interface, and re-exporting it from the Rust binary would
be a lie (the Rust program does not host libc's `stdin` object; it reads fd 0
through `std::io::stdin()`, i.e. `read(2)`, which is exactly what glibc's
`fgets(…, stdin)` does).

Every other entry in either `nm -D` listing is an **undefined** (`U`) or **weak
undefined** (`w`) import from libc/libgcc, i.e. the imports each toolchain needs
for its own runtime:

| side | undefined imports (`U`/`w`) |
|------|------------------------------|
| C    | `__libc_start_main`, `atoi`, `ctime`, `exit`, `fgets`, `printf`, `puts`, `strcmp`, `strcpy`, `strcspn`, `strlen`, `strncmp`, `strncpy`, `strstr`, `strtok`, `time`, `_ITM_*`, `__gmon_start__` |
| Rust | `__libc_start_main`, `ctime`, `exit`, `raise`, `signal`, `strlen`, `time`, `write`, `read`, `malloc`, … (Rust std runtime) plus `_ITM_*`, `__gmon_start__` |

**0 missing/undefined non-libc symbols in the Rust binary** — every `U` entry it
has is a libc/libgcc import resolved by the dynamic loader (verified: the binary
runs). There is no C symbol that the Rust artifact fails to provide.

## 2. Completeness check that *does* apply: source-level function parity

Because no function is exported dynamically, the meaningful anti-"partial
translation" check is the static symbol table of the C binary (`nm -t`, filtered
to symbols coming from `main.c`) against the Rust translation. All 26 functions
defined by `main.c` are present:

| # | C function (`nm`, `main.c`) | Rust counterpart in `src/main.rs` |
|---|------------------------------|-----------------------------------|
| 1 | `parse_command` | `fn parse_command` |
| 2 | `cmd_adduser` | `Mach::cmd_adduser` |
| 3 | `cmd_login` | `Mach::cmd_login` |
| 4 | `cmd_logout` | `Mach::cmd_logout` |
| 5 | `cmd_whoami` | `Mach::cmd_whoami` |
| 6 | `cmd_listusers` | `Mach::cmd_listusers` |
| 7 | `cmd_createfile` | `Mach::cmd_createfile` |
| 8 | `cmd_readfile` | `Mach::cmd_readfile` |
| 9 | `cmd_writefile` | `Mach::cmd_writefile` |
| 10 | `cmd_deletefile` | `Mach::cmd_deletefile` |
| 11 | `cmd_listfiles` | `Mach::cmd_listfiles` |
| 12 | `cmd_set` | `Mach::cmd_set` |
| 13 | `cmd_get` | `Mach::cmd_get` |
| 14 | `cmd_unset` | `Mach::cmd_unset` |
| 15 | `cmd_listvars` | `Mach::cmd_listvars` |
| 16 | `cmd_compare` | `Mach::cmd_compare` |
| 17 | `cmd_compareN` | `Mach::cmd_compare_n` |
| 18 | `cmd_startswith` | `Mach::cmd_startswith` |
| 19 | `cmd_match` | `Mach::cmd_match` |
| 20 | `cmd_help` | `Mach::cmd_help` |
| 21 | `cmd_debug` | `Mach::cmd_debug` |
| 22 | `cmd_verbose` | `Mach::cmd_verbose` |
| 23 | `cmd_status` | `Mach::cmd_status` |
| 24 | `cmd_time` | `Mach::cmd_time` |
| 25 | `process_command` | `Mach::process_command` |
| 26 | `main` | `fn main` |

Non-`main.c` symbols in the C binary (`_start`, `_init`, `_fini`,
`_dl_relocate_static_pie`, `frame_dummy`, `register_tm_clones`,
`deregister_tm_clones`, `__do_global_dtors_aux`) are CRT/toolchain glue, not
translatable source; the Rust binary has the equivalent Rust CRT glue.

The libc functions `main.c` calls (`strcmp`, `strncmp`, `strcpy`, `strncpy`,
`strtok`, `strstr`, `strcspn`, `strlen`, `atoi`, `printf`, `puts`, `fgets`,
`time`, `ctime`, `exit`) are re-implemented in `src/main.rs` with glibc-exact
semantics (`c_strcmp`, `c_strncmp`, `Mach::strcpy_from`, `Mach::strcpy_mem`,
`Mach::strcmp_mem`, `Mach::strcmp_mm`, `parse_command`, `c_strstr_found`,
`c_atoi`, `cfmt`/`pf!`, `fgets`), except `time`/`ctime`, which are called
through FFI so the locale/timezone-dependent output is identical.

## 3. Emulated data layout (why the Rust binary must know the C addresses)

`main.c` `strcpy`s 63-byte tokens into 32-byte struct members, so the reference
program's overruns are *observable* and must be reproduced. The Rust
translation therefore models the reference binary's writable mapping as one flat
array with gcc's exact layout, taken from `nm -n c_src/build/driver`:

| address | object | size |
|---------|--------|------|
| `0x406000` | start of the RW page mapping (`PT_LOAD` RW, page aligned) | — |
| `0x406de8` | `.init_array` / `.fini_array` / `.dynamic` (clobbering them kills `exit()`) | 480 |
| `0x406fc8` | `.got` + `.got.plt` (clobbering them kills the next libc call) | 176 |
| `0x407078` | `.data` (plain data: survives) | 4 |
| `0x40707c` | `__bss_start` | — |
| `0x407080` | `stdin` (copy reloc) | 8 |
| `0x4070a0` | `users[10]` | 720 |
| `0x407370` | `user_count` | 4 |
| `0x407378` | `current_user` | 8 |
| `0x407380` | `files[20]` | 12240 |
| `0x40a350` | `file_count` | 4 |
| `0x40a360` | `variables[20]` | 3200 |
| `0x40afe0` | `variable_count` | 4 |
| `0x40afe4` | `debug_mode` | 4 |
| `0x40afe8` | `verbose_mode` | 4 |
| `0x40aff0` | `_end` | — |
| `0x40b000` | end of the RW page mapping (first faulting address) | — |

Verified against the reference binary: writes that reach `0x40b000` (e.g.
`users[225]`) kill it with `SIGSEGV`, writes at `0x40aff0..0x40b000` do not, the
padding between `completed.0` and `users` is harmless (`ERRORS.md` U08), a write
into `.got.plt` dies at the next libc call (U09) and one into `.dynamic` dies
before `exit()` can flush (U10).

`stdout` is likewise emulated as glibc's stream: fully buffered in
`STDIO_BUFSIZ = 4096` blocks for pipes/files, line buffered for a terminal, with
the line-buffer flush glibc performs before reading from an interactive stdin.
All four stdin/stdout pipe-vs-tty combinations were compared against the
reference binary, with and without a crash (`CONFIGS.md` C51).

## 4. Feature / configuration combinations

`translated_rust/Cargo.toml` declares **no `[features]` section at all**, so the
complete set of feature combinations is:

| # | cargo invocation | meaning |
|---|------------------|---------|
| 1 | `cargo check/test` | default (= no features) |
| 2 | `cargo check/test --no-default-features` | identical: there is no `default` feature to disable |
| 3 | `cargo check/test --all-features` | identical: there are no features |

`c_src/CMakeLists.txt` likewise has no options, no `option()`, no
`target_compile_definitions`, no `#ifdef` in `main.c` (verified: `grep -c
'#if' c_src/src/main.c` → 0), and no `CMAKE_BUILD_TYPE` default, so the C side
compiles at `-O0` with no conditional code. There is exactly **one** build
configuration on both sides; `tests/parity.rs::d05_single_build_configuration`
asserts this mechanically so the claim cannot silently rot.

## 5. How the verification is reproduced

```
./verify.sh          # C build + cargo check/test for every configuration
                     # + the release profile + the nm -D symbol diff
```

| file | phase | contents |
|------|-------|----------|
| `tests/common/mod.rs` | — | harness: builds/locates both binaries, runs them over the process boundary (stdin -> stdout/stderr/exit status), byte-compares (with `ctime` normalisation), deterministic xorshift RNG |
| `tests/configs.rs` | B | 51 tests, one per `CONFIGS.md` row, randomized with fixed seeds |
| `tests/errors.rs` | C | 73 tests, one per `ERRORS.md` row (`E01`–`E62`, `U01`–`U11`); each also asserts that the reference binary really emitted that rejection / exit status |
| `tests/fuzz.rs` | B/C | 3 broad randomized sweeps (210 scripts) mixing every axis |
| `tests/parity.rs` | D | symbol parity, `dlopen` inapplicability (via `libloading`), C-function-to-Rust-function parity, single-build-configuration assertions, release-profile equivalence |

Additionally, 4500+ randomized cases (including streams of uniformly random
bytes) were compared outside the test suite, and the harness itself was
mutation-tested: injecting a wrong message, a wrong stdio block size, a
`-1/0/+1` `strcmp` convention, a cached `user_count`, a wrong mapping size or a
`fgets` off-by-one each made the suite fail, so the tests are not vacuous.
