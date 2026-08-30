//! Differential tests: run the C program from `c_src/` and the Rust program
//! from this crate as subprocesses on the same stdin, and require that stdout,
//! stderr and the exit status are identical.
//!
//! The Rust binary is never called as a library; it is driven exactly the way a
//! shell would drive it, because that is what the C program is compared
//! against.
//!
//! The case list is derived from reading `c_src/src/main.c` and
//! `c_src/src/stb_perlin.h`; see `enumerate()` for the branch-by-branch
//! rationale.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------- locating the
// two programs

fn workspace_root() -> PathBuf {
    // .../<workdir>/translation/Cargo.toml -> <workdir>
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

/// Path to the compiled C program, building it if necessary.
///
/// A pre-existing `c_src/build/driver` is used as-is. Otherwise the C sources
/// are configured and built out-of-tree into `translation/target/c_build`, so
/// that `c_src/` itself is never written to.
fn c_driver() -> PathBuf {
    let root = workspace_root();
    let prebuilt = root.join("c_src").join("build").join("driver");
    if prebuilt.is_file() {
        return prebuilt;
    }

    let src = root.join("c_src");
    let build = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("c_build");
    let out = build.join("driver");
    if out.is_file() {
        return out;
    }

    std::fs::create_dir_all(&build).expect("create c_build dir");
    let configure = Command::new("cmake")
        .arg("-S")
        .arg(&src)
        .arg("-B")
        .arg(&build)
        .output()
        .expect("run cmake (is cmake installed?)");
    assert!(
        configure.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&configure.stdout),
        String::from_utf8_lossy(&configure.stderr)
    );
    let compile = Command::new("cmake")
        .arg("--build")
        .arg(&build)
        .output()
        .expect("run cmake --build");
    assert!(
        compile.status.success(),
        "cmake --build failed:\n{}\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );
    assert!(out.is_file(), "cmake produced no driver at {}", out.display());
    out
}

fn rust_driver() -> PathBuf {
    // Cargo builds the crate's bin target before running integration tests and
    // hands us its path, so this is always the binary matching the sources.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

// ------------------------------------------------------------------ running one

/// Everything about a run that the C program can be observed to produce.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signum)` when killed by a signal.
    /// A process *killed by* signal N is deliberately distinguished from one
    /// that *exits with* code 128+N -- they are different wait statuses.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match self.status {
            Ok(c) => format!("exit {c}"),
            Err(s) => format!("killed by signal {s}"),
        };
        write!(
            f,
            "{{ stdout: {:?}, stderr: {:?}, {} }}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            status
        )
    }
}

fn run(exe: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));

    // Write on a helper thread: some inputs are larger than a pipe buffer, and
    // the child may exit without draining stdin (a broken pipe is not an error
    // here -- the C program does the same thing).
    let mut stdin = child.stdin.take().expect("piped stdin");
    let data = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
        let _ = stdin.flush();
    });

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("wait for {}: {e}", exe.display()));
    let _ = writer.join();

    #[cfg(unix)]
    let status = {
        use std::os::unix::process::ExitStatusExt;
        match out.status.signal() {
            // Note: the core-dump flag of the raw wait status is deliberately
            // not compared; it depends on the ambient RLIMIT_CORE, not on the
            // program.
            Some(sig) => Err(sig),
            None => Ok(out.status.code().unwrap_or(-1)),
        }
    };
    #[cfg(not(unix))]
    let status = Ok(out.status.code().unwrap_or(-1));

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

// -------------------------------------------------------------- the input space

/// A deterministic xorshift, so the randomised sweep is reproducible without
/// pulling in a dependency.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
    /// A float in `-range..range` with a few decimals.
    fn coord(&mut self, range: f64) -> String {
        let v = (self.next_u64() % 2_000_001) as f64 / 1_000_000.0 - 1.0;
        format!("{:.6}", v * range)
    }
}

/// Whitespace-free numeric tokens that exercise `scanf`'s `%f` conversion.
const FLOAT_TOKENS: &[&str] = &[
    // ordinary
    "0", "-0.0", "1", "-1", "0.5", "-0.5", "1.0000001", "0.9999999", "255.5",
    "256", "-256.5", "-1e-30", "16777216", "16777217", "-16777216",
    // at and beyond the int32 range that `(int)` truncation cares about
    "2147483647", "2147483648", "-2147483648", "-2147483649",
    // at and beyond the f32 range
    "1e38", "3.4028235e38", "3.4028236e38", "1e39", "-1e39", "1e-45", "-1e-45",
    "1e-46", "1e300", "-1e300",
    // specials glibc's %f accepts
    "nan", "-nan", "NaN", "NAN", "inf", "-inf", "INF", "Inf", "infinity",
    "INFINITY", "+inf", "+nan",
    // accepted, but the (n-char-sequence) is left unread
    "nan(", "nan()", "nan(0x1)",
    // matching failures
    ".", "-.", "+.", "in", "infi", "infin", "infinit", "inf1", "0x", "0X",
    "0xg", "x", "-", "+", "e5", "--1", "++1", "1_0",
    // forms where the conversion succeeds but leaves characters behind
    ".5", "5.", "-.5", "+.5", "0.", "1e", "1E", "1e+", "1e-", "1.5e", "1.5e-",
    "1-", "1+", "1.2.3", "1e1e1",
    // exponents and hex floats
    "1e5", "1e-5", "1e+5", "1e05", "0e0", "1e400", "-1e400", "1e-400",
    "0x0", "0x1", "0x10", "0x1.8p1", "0x1.8p+1", "0x1.8p-1", "0x.8p1", "0x1p",
    "0x1p+", "0xp1", "0x1P2", "0x1p-1",
];

/// Tokens that exercise `scanf`'s `%d` conversion, including glibc's
/// `strtol`-style saturation followed by truncation to `int`.
const INT_TOKENS: &[&str] = &[
    "0", "1", "-1", "+1", "007", "-0", "00",
    "2147483647", "2147483648", "-2147483648", "-2147483649", "4294967296",
    "9223372036854775807", "9223372036854775808", "-9223372036854775808",
    "-9223372036854775809", "99999999999999999999999999",
    "-99999999999999999999999999",
    "x", "-", "+", ".", "1.5", "1e5", "--1", "0x10",
];

fn enumerate() -> Vec<(String, Vec<u8>)> {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    let mut push = |group: &str, input: &[u8]| {
        cases.push((group.to_string(), input.to_vec()));
    };

    // ---- main.c: the `which` switch, including the `default: return NAN` arm.
    for w in [
        "-2147483648", "-100", "-2", "-1", "0", "1", "2", "3", "4", "5", "6",
        "7", "100", "2147483647",
    ] {
        push(
            "which switch arms",
            format!("{w} 0.5 0.25 0.125 3 5 7 9 2 0.5 1 4").as_bytes(),
        );
    }

    // ---- main.c: scanf reads 12 conversions and its return value is ignored,
    // so every prefix length is a distinct behaviour (the unconverted variables
    // keep their zero initialisers). Empty input is the 0-conversion case.
    let full = ["1", "0.5", "0.25", "0.125", "4", "8", "16", "33", "2", "0.5", "1", "4"];
    for k in 0..=full.len() {
        for suffix in ["", " ", "\n", "\t", "\r\n", "  \n  "] {
            push(
                "short input / prefix of the conversions",
                format!("{}{suffix}", full[..k].join(" ")).as_bytes(),
            );
        }
    }

    // ---- main.c: a single item, and nothing but whitespace.
    for s in ["", " ", "\n", "\t", "\x0b", "\x0c", "\r", "\n\n\n", "   \t\n\x0b\x0c\r  ", "3", "0"] {
        push("empty / whitespace-only / single item", s.as_bytes());
    }

    // ---- scanf: %f matching failure or partial match in each of the four
    // float slots (x, y, z, lacunarity, gain, offset).
    for slot in [1usize, 2, 3, 8, 9, 10] {
        for t in FLOAT_TOKENS {
            let mut toks = full;
            toks[slot] = t;
            push("scanf %f token", toks.join(" ").as_bytes());
        }
    }

    // ---- scanf: %d matching failure / overflow in each int slot.
    for slot in [0usize, 4, 5, 6, 7, 11] {
        for t in INT_TOKENS {
            let mut toks = full;
            toks[slot] = t;
            push("scanf %d token", toks.join(" ").as_bytes());
        }
    }

    // ---- scanf skips whitespace before every conversion, so it reads straight
    // across newlines; and it stops after 12 conversions, ignoring the rest.
    for s in [
        "0\n0.5\n0.25\n0.125\n0\n0\n0\n0\n0\n0\n0\n0\n",
        "0\r\n0.5\r\n0.25\r\n0.125\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n",
        "  \t\n 1 \t 0.5 0.5 0.5 0 0 0 3 0 0 0 0",
        "2\x0b0.5\x0c0.5\r0.5 0 0 0 0 2 0.5 1 6",
        "0 0.5 0.25 0.125 0 0 0 0 0 0 0 0 extra junk",
        "0 0.5 0.25 0.125 0 0 0 0 0 0 0 0\n9 9 9 9 9 9 9 9 9 9 9 9\n",
    ] {
        push("whitespace / trailing input", s.as_bytes());
    }

    // ---- non-text bytes on stdin.
    for b in [
        &b"\x00"[..],
        &b"\xff\xfe"[..],
        &b"0\x000.5 0.5 0.5 0 0 0 0 0 0 0 0"[..],
        &b"0 1 1 1 0 0 0 0 0 0 0 0\x00"[..],
        &b"\x000 1 1 1 0 0 0 0 0 0 0 0"[..],
    ] {
        push("non-text bytes", b);
    }

    // ---- stb__perlin_fastfloor: `(a < ai) ? ai-1 : ai` -- the taken branch
    // needs a negative fraction; the `(int)a` conversion itself has to cope with
    // NaN and with values outside the int range.
    for f in FLOAT_TOKENS {
        for which in ["0", "1", "5"] {
            push(
                "fastfloor / (int) conversion",
                format!("{which} {f} {f} {f} 0 0 0 7 2 0.5 1 3").as_bytes(),
            );
        }
        push(
            "fastfloor per-coordinate",
            format!("0 {f} 0.5 0.5 0 0 0 0 0 0 0 0").as_bytes(),
        );
        push(
            "fastfloor per-coordinate",
            format!("0 0.5 {f} 0.5 0 0 0 0 0 0 0 0").as_bytes(),
        );
        push(
            "fastfloor per-coordinate",
            format!("0 0.5 0.5 {f} 0 0 0 0 0 0 0 0").as_bytes(),
        );
    }

    // ---- stb_perlin_noise3_internal: x_mask = (x_wrap-1) & 255. wrap 0 gives a
    // full 255 mask, wrap 1 gives a zero mask, and non-powers-of-two give the
    // "wrong" masks the documentation warns about. Negative wraps and INT_MIN
    // exercise the wrapping subtraction.
    for w in [
        "-2147483648", "-256", "-4", "-1", "0", "1", "2", "3", "5", "7", "16",
        "255", "256", "257", "512", "1024", "65536", "2147483647",
    ] {
        push(
            "noise3 wrap masks",
            format!("0 12.75 -3.5 8.125 {w} {w} {w} 0 0 0 0 0").as_bytes(),
        );
        push(
            "noise3 wrap masks",
            format!("1 12.75 -3.5 8.125 {w} 16 {w} 77 0 0 0 0").as_bytes(),
        );
        push(
            "noise3 wrap masks",
            format!("0 -20.5 33.25 -0.75 {w} 4 2 0 0 0 0 0").as_bytes(),
        );
    }

    // ---- stb_perlin_noise3_seed: only the low 8 bits of `seed` survive the
    // `(unsigned char)` cast, so seeds around the byte boundaries and negative
    // seeds all have to fold the same way.
    for s in [
        "-2147483648", "-257", "-256", "-255", "-129", "-128", "-1", "0", "1",
        "127", "128", "255", "256", "257", "511", "512", "2147483647",
    ] {
        push(
            "seed truncated to unsigned char",
            format!("1 0.5 0.25 0.125 0 0 0 {s} 0 0 0 0").as_bytes(),
        );
        push(
            "seed truncated to unsigned char",
            format!("5 -12.25 7.5 -3.125 6 10 14 {s} 0 0 0 0").as_bytes(),
        );
    }

    // ---- the three fractal functions: the loop body never runs for
    // octaves <= 0 (sum stays 0.0f), and the octave counter is cast to
    // `unsigned char` for the seed, so it folds every 256 octaves.
    for which in ["2", "3", "4"] {
        for o in [
            "-2147483648", "-100", "-1", "0", "1", "2", "3", "5", "6", "10",
            "255", "256", "257", "300", "512", "513",
        ] {
            push(
                "fractal octave counts",
                format!("{which} 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 {o}").as_bytes(),
            );
        }
        // lacunarity / gain / offset extremes: overflow to inf, collapse to
        // zero, sign flips, and NaN propagation through fabs()/multiplication.
        for lac in ["0", "-0.0", "1", "-1", "2", "1e20", "-1e20", "inf", "-inf", "nan", "1e-40"] {
            for gain in ["0", "1", "0.5", "-1", "2", "inf", "-inf", "nan"] {
                push(
                    "fractal lacunarity/gain",
                    format!("{which} 0.5 0.5 0.5 0 0 0 0 {lac} {gain} 1 5").as_bytes(),
                );
            }
        }
    }
    // ridge's `offset - fabs(r)` then `r*r`: drive the result across the whole
    // printable magnitude range, including inf and NaN.
    for off in [
        "0", "-0.0", "-1", "1", "1e-20", "1e-10", "0.1", "10", "1e9", "1e10",
        "1e19", "1e20", "1e30", "1e38", "inf", "-inf", "nan",
    ] {
        for oct in ["1", "2", "6"] {
            push(
                "ridge offset (printf magnitude sweep)",
                format!("2 0.5 0.5 0.5 0 0 0 0 2 0.5 {off} {oct}").as_bytes(),
            );
        }
    }

    // ---- printf("%.9g"): %e style vs %f style, the exponent boundaries
    // (X < -4 and X >= 9), trailing-zero stripping, and the sign of zero/NaN.
    for m in [
        "1e-45", "1e-44", "1e-40", "1e-38", "1.17549435e-38", "1e-30", "1e-20",
        "1e-10", "1e-8", "1e-7", "1e-6", "1e-5", "1e-4", "1e-3", "0.1",
        "0.999999999", "1", "1.00000001", "9.99999999", "10", "99.9999999",
        "100", "1000", "12345.6789", "99999999", "100000000", "999999999",
        "1e9", "1.23456789e9", "1e10", "1e15", "1e20", "1e30", "1e38",
        "3.4028235e38", "1e39",
    ] {
        for sign in ["", "-"] {
            push(
                "printf %.9g magnitudes",
                format!("3 0.5 0.5 0.5 0 0 0 0 {sign}{m} 1 0 1").as_bytes(),
            );
            push(
                "printf %.9g magnitudes",
                format!("4 0.5 0.5 0.5 0 0 0 0 2 {sign}{m} 0 3").as_bytes(),
            );
            push(
                "printf %.9g magnitudes",
                format!("2 {sign}{m} 0.5 0.5 0 0 0 0 2 0.5 1 4").as_bytes(),
            );
        }
    }
    // negative zero reaches printf as "-0"
    push("printf negative zero", b"5 0 -1 -0.0 0 0 0 0 2 0.5 1 3");

    // ---- NaN/inf sign: gcc emits addss/subss/mulss whose result for a NaN
    // operand is the *destination* register, which decides whether printf prints
    // "nan" or "-nan". Cover every combination of specials per `which`.
    let specials = [
        "nan", "-nan", "inf", "-inf", "0", "-0.0", "1", "-1", "1e20", "-1e20",
        "0.5", "-0.5",
    ];
    for which in ["0", "1", "2", "3", "4", "5"] {
        for x in specials {
            for y in specials {
                for z in specials {
                    push(
                        "NaN sign / SSE operand order",
                        format!("{which} {x} {y} {z} 0 0 0 0 2 0.5 1 3").as_bytes(),
                    );
                }
            }
        }
    }

    // ---- stb_perlin_noise3_wrap_nonpow2: `wrap ? wrap : 256`, the `x0 < 0`
    // fixups, and `(x0+1) % wrap`.
    for w in ["0", "1", "2", "3", "5", "6", "7", "10", "100", "128", "255", "256"] {
        for coord in ["0.5", "-0.5", "5.5", "-5.5", "255.5", "-255.5", "1000.5", "-1000.5", "0", "-1"] {
            push(
                "wrap_nonpow2 in-range wraps",
                format!("5 {coord} 0.5 0.5 {w} 0 0 0 0 0 0 0").as_bytes(),
            );
            push(
                "wrap_nonpow2 in-range wraps",
                format!("5 0.5 {coord} 0.5 0 {w} 0 0 0 0 0 0").as_bytes(),
            );
            push(
                "wrap_nonpow2 in-range wraps",
                format!("5 0.5 0.5 {coord} 0 0 {w} 0 0 0 0 0").as_bytes(),
            );
            push(
                "wrap_nonpow2 in-range wraps",
                format!("5 {coord} {coord} {coord} {w} {w} {w} 200 0 0 0 0").as_bytes(),
            );
        }
    }

    // ---- wrap_nonpow2 with a wrap larger than a table: the C code reads past
    // the end of stb__perlin_randtab into the objects that follow it, and past
    // the last mapped page it dies of SIGSEGV. A wrap of 2000000000 makes the
    // table index equal to floor(x), so each of these picks one exact offset.
    for p in [
        "256", "300", "400", "500", "511", "512", "513", "700", "704", "705",
        "1000", "1215", "1216", "1217", "2000", "3000", "4000", "4029", "4030",
        "4031", "4032", "4033", "4100", "5000", "65536", "1000000", "16777216",
    ] {
        push(
            "wrap_nonpow2 reads past the tables",
            format!("5 {p}.5 0.5 0.5 2000000000 0 0 0 0 0 0 0").as_bytes(),
        );
    }
    for w in [
        "257", "258", "270", "288", "320", "400", "512", "600", "700", "736",
        "768", "1000", "1024", "2000", "4096", "65536", "1000000", "2147483647",
    ] {
        for coord in ["0.5", "1000.5", "12345.5", "100000.5"] {
            push(
                "wrap_nonpow2 reads past the tables",
                format!("5 {coord} 0.5 0.5 {w} 0 0 0 0 0 0 0").as_bytes(),
            );
            push(
                "wrap_nonpow2 reads past the tables",
                format!("5 0.5 {coord} 0.5 0 {w} 0 0 0 0 0 0").as_bytes(),
            );
            push(
                "wrap_nonpow2 reads past the tables",
                format!("5 0.5 0.5 {coord} 0 0 {w} 0 0 0 0 0").as_bytes(),
            );
            push(
                "wrap_nonpow2 reads past the tables",
                format!("5 {coord} {coord} {coord} {w} {w} {w} 137 0 0 0 0").as_bytes(),
            );
        }
    }
    // Small negative wraps: `x0 += x_wrap2` leaves x0 in (2*wrap, wrap], which
    // for these values stays inside the zero padding in front of the table.
    for w in ["-1", "-2", "-3", "-4", "-8", "-16"] {
        for coord in ["0.5", "5.5", "1000.5", "0", "1"] {
            push(
                "wrap_nonpow2 small negative wraps",
                format!("5 {coord} 0.5 0.5 {w} 0 0 0 0 0 0 0").as_bytes(),
            );
            push(
                "wrap_nonpow2 small negative wraps",
                format!("5 0.5 {coord} 0.5 0 {w} 0 0 0 0 0 0").as_bytes(),
            );
            push(
                "wrap_nonpow2 small negative wraps",
                format!("5 0.5 0.5 {coord} 0 0 {w} 0 0 0 0 0").as_bytes(),
            );
        }
    }

    // ---- wrap_nonpow2 with a wrap of -1 and floor(x) == INT_MIN: `INT_MIN % -1`
    // makes the `idiv` gcc emits raise SIGFPE, so the program dies having
    // printed nothing.
    for coord in [
        "nan", "-nan", "inf", "-inf", "1e20", "-1e20", "3e38", "-3e38",
        "2147483648", "-2147483649", "-2147483648",
    ] {
        push(
            "wrap_nonpow2 INT_MIN % -1 traps",
            format!("5 {coord} 0.5 0.5 -1 0 0 0 0 0 0 0").as_bytes(),
        );
        push(
            "wrap_nonpow2 INT_MIN % -1 traps",
            format!("5 0.5 {coord} 0.5 0 -1 0 0 0 0 0 0").as_bytes(),
        );
        push(
            "wrap_nonpow2 INT_MIN % -1 traps",
            format!("5 0.5 0.5 {coord} 0 0 -1 0 0 0 0 0").as_bytes(),
        );
        push(
            "wrap_nonpow2 INT_MIN % -1 traps",
            format!("5 {coord} {coord} {coord} -1 -1 -1 0 0 0 0 0").as_bytes(),
        );
        // ...whereas noise3_internal masks instead of dividing, so the same
        // input is harmless there.
        push(
            "noise3 masks instead of dividing",
            format!("0 {coord} {coord} {coord} -1 -1 -1 0 0 0 0 0").as_bytes(),
        );
    }

    // ---- inputs far larger than a stdio buffer.
    let mut big = |name: &str, v: Vec<u8>| cases.push((name.to_string(), v));
    big("oversized input", [b" ".repeat(100_000), b"0 0.5 0.5 0.5 0 0 0 0 0 0 0 0".to_vec()].concat());
    big("oversized input", [b"0 0.5 0.5 0.5 0 0 0 0 0 0 0 0".to_vec(), b"\n".repeat(200_000)].concat());
    big("oversized input", [b"0 ".to_vec(), b"0".repeat(100_000), b".5 0.5 0.5 0 0 0 0 0 0 0 0".to_vec()].concat());
    big("oversized input", [b"0 0.".to_vec(), b"0".repeat(50_000), b"5 0.5 0.5 0 0 0 0 0 0 0 0".to_vec()].concat());
    big("oversized input", [b"9".repeat(100_000), b" 1 1 1 0 0 0 0 0 0 0 0".to_vec()].concat());
    big("oversized input", [b"0 1e".to_vec(), b"9".repeat(5_000), b" 1 1 0 0 0 0 0 0 0 0".to_vec()].concat());
    big("oversized input", [b"0 1e-".to_vec(), b"9".repeat(5_000), b" 1 1 0 0 0 0 0 0 0 0".to_vec()].concat());
    big("oversized input", [b"0 0x".to_vec(), b"f".repeat(5_000), b" 1 1 0 0 0 0 0 0 0 0".to_vec()].concat());
    big("oversized input", [b"0 0.5 0.5 0.5 0 0 0 0 0 0 0 0 ".to_vec(), b"x".repeat(100_000)].concat());

    // ---- randomised sweep over the well-defined part of the input space, to
    // catch anything the hand-written classes above miss.
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let wraps = ["0", "1", "2", "3", "4", "5", "7", "8", "16", "32", "64", "100", "128", "255", "256"];
    for _ in 0..1500 {
        let which = rng.below(7) as i64 - 1;
        let (x, y, z) = (rng.coord(300.0), rng.coord(300.0), rng.coord(300.0));
        let (xw, yw, zw) = (*rng.pick(&wraps), *rng.pick(&wraps), *rng.pick(&wraps));
        let seed = rng.below(1200) as i64 - 600;
        let (lac, gain, off) = (rng.coord(4.0), rng.coord(2.0), rng.coord(4.0));
        let oct = rng.below(12) as i64 - 2;
        cases.push((
            "randomised sweep".to_string(),
            format!("{which} {x} {y} {z} {xw} {yw} {zw} {seed} {lac} {gain} {off} {oct}")
                .into_bytes(),
        ));
    }
    for _ in 0..600 {
        // same, but with specials mixed into the float slots
        let pick = |r: &mut Rng| -> String {
            if r.below(2) == 0 {
                r.coord(1e20)
            } else {
                (*r.pick(&specials)).to_string()
            }
        };
        let which = rng.below(7) as i64 - 1;
        let (x, y, z) = (pick(&mut rng), pick(&mut rng), pick(&mut rng));
        let (lac, gain, off) = (pick(&mut rng), pick(&mut rng), pick(&mut rng));
        let seed = rng.below(512) as i64;
        let oct = rng.below(10) as i64;
        cases.push((
            "randomised sweep with specials".to_string(),
            format!("{which} {x} {y} {z} 0 0 0 {seed} {lac} {gain} {off} {oct}").into_bytes(),
        ));
    }

    cases
}

// ------------------------------------------------------------------- the test

#[test]
fn c_and_rust_agree_on_stdout_stderr_and_exit_status() {
    let c = c_driver();
    let rust = rust_driver();

    let cases = enumerate();
    let mut per_group: std::collections::BTreeMap<&str, usize> = Default::default();
    for (group, _) in &cases {
        *per_group.entry(group.as_str()).or_insert(0) += 1;
    }
    eprintln!(
        "comparing {} inputs across {} classes:",
        cases.len(),
        per_group.len()
    );
    for (g, n) in &per_group {
        eprintln!("  {n:>5}  {g}");
    }

    // Each case is four process spawns' worth of latency, so fan the corpus out
    // over a few threads. The comparison itself is independent per case.
    let threads = std::thread::available_parallelism()
        .map(|n| n.get().min(16))
        .unwrap_or(4);
    let cases = std::sync::Arc::new(cases);
    let next = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handles = Vec::new();
    for _ in 0..threads {
        let cases = std::sync::Arc::clone(&cases);
        let next = std::sync::Arc::clone(&next);
        let (c, rust) = (c.clone(), rust.clone());
        handles.push(std::thread::spawn(move || {
            let mut failures: Vec<String> = Vec::new();
            loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if i >= cases.len() {
                    break;
                }
                let (group, input) = &cases[i];
                let want = run(&c, input);
                let got = run(&rust, input);
                if want != got {
                    failures.push(format!(
                        "[{group}] stdin = {:?}\n     C: {want:?}\n  rust: {got:?}",
                        String::from_utf8_lossy(&input[..input.len().min(160)])
                    ));
                }
            }
            failures
        }));
    }
    let mut failures: Vec<String> = Vec::new();
    for h in handles {
        failures.extend(h.join().expect("comparison thread panicked"));
    }
    failures.sort();

    let total = failures.len();
    failures.truncate(25);
    assert!(
        failures.is_empty(),
        "{} of {} inputs differ (first {} shown):\n{}",
        total,
        cases.len(),
        failures.len(),
        failures.join("\n")
    );
}

/// Guards the harness itself: if these two invariants broke, the comparison
/// above would be comparing nothing.
#[test]
fn harness_actually_runs_two_distinct_programs() {
    let c = c_driver();
    let rust = rust_driver();
    assert_ne!(c, rust);
    assert!(c.is_file(), "no C binary at {}", c.display());
    assert!(rust.is_file(), "no Rust binary at {}", rust.display());

    // Both must produce the documented happy-path output, so a silent failure to
    // start (e.g. an empty stdout from a crash) cannot pass as agreement.
    let input = b"0 0.5 0.25 0.125 0 0 0 0 0 0 0 0";
    for exe in [&c, &rust] {
        let o = run(exe, input);
        assert_eq!(o.status, Ok(0), "{} exit status", exe.display());
        assert_eq!(o.stderr, b"", "{} stderr", exe.display());
        assert_eq!(
            String::from_utf8_lossy(&o.stdout),
            "-0.142593384\n",
            "{} stdout",
            exe.display()
        );
    }
}
