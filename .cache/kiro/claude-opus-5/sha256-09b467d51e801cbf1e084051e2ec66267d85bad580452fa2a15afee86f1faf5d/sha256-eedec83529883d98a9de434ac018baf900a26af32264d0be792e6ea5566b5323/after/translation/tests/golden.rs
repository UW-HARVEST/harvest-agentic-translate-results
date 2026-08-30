//! Golden tests: every outcome below was captured from the original C program in
//! `c_src/` (built with the supplied CMakeLists, i.e. `gcc` with no optimisation
//! flags) and must be reproduced byte for byte.
//!
//! `tests/differential.rs` is the authoritative comparison -- it runs the real C
//! program. This file pins the same behaviour without needing a C toolchain, so a
//! regression is still caught when `c_src/` cannot be built. Both stderr and the
//! exit status are pinned, not just stdout: several inputs make the C program die
//! of a signal after printing nothing, and a test that only looked at stdout
//! would accept a Rust program that exited 0 instead.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

struct Case {
    stdin: &'static [u8],
    stdout: &'static [u8],
    stderr: &'static [u8],
    /// `Ok(code)` for a normal exit, `Err(signum)` when killed by that signal.
    status: Result<i32, i32>,
}

const CASES: &[Case] = &[
    Case {
        stdin: b"0 0.5 0.25 0.125 0 0 0 0 0 0 0 0",
        stdout: b"-0.142593384\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 12.75 -3.5 8.125 16 16 16 0 0 0 0 0",
        stdout: b"-0.516644061\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"1 0.5 0.25 0.125 0 0 0 7 0 0 0 0",
        stdout: b"-0.076374203\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"1 -20.5 33.25 -0.75 8 4 2 255 0 0 0 0",
        stdout: b"0.448242188\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"2 0.5 0.5 0.5 0 0 0 0 2 0.5 1 6",
        stdout: b"0.421875\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"2 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 8",
        stdout: b"0.620870948\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"3 0.5 0.5 0.5 0 0 0 0 2 0.5 0 6",
        stdout: b"-0.5\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"3 -7.125 4.5 0.875 0 0 0 0 2.5 0.4 0 10",
        stdout: b"0.09392827\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"4 0.5 0.5 0.5 0 0 0 0 2 0.5 0 6",
        stdout: b"0.5\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"4 9.25 -1.5 6.75 0 0 0 0 1.75 0.6 0 5",
        stdout: b"0.87352705\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"5 0.5 0.5 0.5 0 0 0 0 0 0 0 0",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"5 -12.25 7.5 -3.125 6 10 14 200 0 0 0 0",
        stdout: b"0.402987331\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"5 100.5 -100.5 50.25 3 5 7 9 0 0 0 0",
        stdout: b"0.118530273\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"-1 1 2 3 0 0 0 0 0 0 0 0",
        stdout: b"nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"6 1 2 3 0 0 0 0 0 0 0 0",
        stdout: b"nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"-2147483648 1 2 3 0 0 0 0 0 0 0 0",
        stdout: b"nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"2147483647 1 2 3 0 0 0 0 0 0 0 0",
        stdout: b"nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b" ",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"\n",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"\t",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"3",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 1 2",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"abc",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 1e 2 3 0 0 0 5 2 .5 1 4",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 1e- 2 3 0 0 0 5 2 .5 1 4",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 .5 1. -.25 0 0 0 5 2 .5 1 4",
        stdout: b"0.120605469\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 0x10 0x1p-1 1 0 0 0 0 0 0 0 0",
        stdout: b"0.25\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 0x1.8p+1 0.5 0.5 0 0 0 0 0 0 0 0",
        stdout: b"-0.125\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 0x.8p1 0.5 0.5 0 0 0 0 0 0 0 0",
        stdout: b"-0.625\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 nan 0 0 0 0 0 0 0 0 0 0",
        stdout: b"nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 inf 1 1 0 0 0 0 0 0 0 0",
        stdout: b"-nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 -inf 1 1 0 0 0 0 0 0 0 0",
        stdout: b"-nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 infinity 1 1 0 0 0 0 0 0 0 0",
        stdout: b"-nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 infi 1 1 0 0 0 0 0 0 0 0",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 nan(x) 1 1 0 0 0 0 0 0 0 0",
        stdout: b"nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"2 -1e+20 0.0773796436 14.8969174 1 4 2 2 -2 0 1 8",
        stdout: b"-nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"3 1e20 1 1 0 0 0 0 2 0 0 4",
        stdout: b"-nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"4 1e20 1 1 0 0 0 0 2 0 0 4",
        stdout: b"nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 nan 1 1 0 0 0 0 0 0 0 0",
        stdout: b"nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 -nan 1 1 0 0 0 0 0 0 0 0",
        stdout: b"-nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"5 nan 1 1 0 0 0 0 0 0 0 0",
        stdout: b"nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"2 0.5 0.5 0.5 0 0 0 0 2 0.5 1e-20 1",
        stdout: b"0.125\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"2 0.5 0.5 0.5 0 0 0 0 2 0.5 1e10 1",
        stdout: b"5.0000001e+19\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"2 0.5 0.5 0.5 0 0 0 0 2 0.5 1e19 1",
        stdout: b"4.99999984e+37\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"2 0.5 0.5 0.5 0 0 0 0 2 0.5 1e20 1",
        stdout: b"inf\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"2 0.5 0.5 0.5 0 0 0 0 2 0.5 inf 1",
        stdout: b"inf\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"3 1e-45 1 1 0 0 0 0 2 0.5 0 1",
        stdout: b"-1.40129846e-45\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"5 0 -1 -0.0 0 0 0 0 2 0.5 1 3",
        stdout: b"-0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 -0.0 -0.0 -0.0 0 0 0 0 0 0 0 0",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"99999999999999999999 1 2 3 0 0 0 0 0 0 0 0",
        stdout: b"nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"2147483648 1 2 3 0 0 0 0 0 0 0 0",
        stdout: b"nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 -2147483648 0 0 0 0 0 0 0 0 0 0",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"1 0.5 0.5 0.5 0 0 0 9999999999999999999999 0 0 0 0",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0\n0.5\n0.25\n0.125\n0\n0\n0\n0\n0\n0\n0\n0\n",
        stdout: b"-0.142593384\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"  \t\n 1 \t 0.5 0.5 0.5 0 0 0 3 0 0 0 0",
        stdout: b"-0.125\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"2 1 1 1 0 0 0 0 2 0.5 1 0",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"3 1 1 1 0 0 0 0 2 0.5 0 -1",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"4 1 1 1 0 0 0 0 2 0.5 0 -2147483648",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"3 0.25 0.5 0.75 0 0 0 0 1 1 0 256",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"3 0.25 0.5 0.75 0 0 0 0 1 1 0 257",
        stdout: b"-0.0672974586\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 12.75 -3.5 8.125 1 1 1 0 0 0 0 0",
        stdout: b"-0.108947754\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 12.75 -3.5 8.125 3 5 7 0 0 0 0 0",
        stdout: b"0.108947754\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 12.75 -3.5 8.125 -1 -1 -1 0 0 0 0 0",
        stdout: b"-0.0375366211\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"0 12.75 -3.5 8.125 512 512 512 0 0 0 0 0",
        stdout: b"0.391451538\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"1 0.5 0.25 0.125 0 0 0 256 0 0 0 0",
        stdout: b"-0.142593384\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"1 0.5 0.25 0.125 0 0 0 -1 0 0 0 0",
        stdout: b"0.0128701031\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"5 0.5 0.5 0.5 1 1 1 0 0 0 0 0",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"5 -5.5 -5.5 -5.5 6 10 14 0 0 0 0 0",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"5 0.5 0.5 0.5 512 512 512 0 0 0 0 0",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"5 0.5 0.5 0.5 -4 -4 -4 0 0 0 0 0",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"5 1000.5 0.5 0.5 2000000000 0 0 0 0 0 0 0",
        stdout: b"-0.25\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"5 4030.5 0.5 0.5 2000000000 0 0 0 0 0 0 0",
        stdout: b"-0.25\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"5 4031.5 0.5 0.5 2000000000 0 0 0 0 0 0 0",
        stdout: b"",
        stderr: b"",
        status: Err(11),
    },
    Case {
        stdin: b"5 nan 0.5 0.5 -1 0 0 0 0 0 0 0",
        stdout: b"",
        stderr: b"",
        status: Err(8),
    },
    Case {
        stdin: b"5 0.5 nan 0.5 0 -1 0 0 0 0 0 0",
        stdout: b"",
        stderr: b"",
        status: Err(8),
    },
    Case {
        stdin: b"5 0.5 0.5 nan 0 0 -1 0 0 0 0 0",
        stdout: b"",
        stderr: b"",
        status: Err(8),
    },
    Case {
        stdin: b"0 nan nan nan -1 -1 -1 0 0 0 0 0",
        stdout: b"nan\n",
        stderr: b"",
        status: Ok(0),
    },
    Case {
        stdin: b"3 0.001 0.002 0.003 1.0000001 0.9999999 1 20",
        stdout: b"0\n",
        stderr: b"",
        status: Ok(0),
    },
];

fn driver() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn describe(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

#[test]
fn recorded_c_outcomes_are_reproduced() {
    let exe = driver();
    let mut failures = Vec::new();

    for case in CASES {
        let mut child = Command::new(&exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn driver");
        {
            let mut stdin = child.stdin.take().expect("piped stdin");
            let data = case.stdin.to_vec();
            std::thread::spawn(move || {
                let _ = stdin.write_all(&data);
            });
        }
        let out = child.wait_with_output().expect("run driver");

        #[cfg(unix)]
        let status = {
            use std::os::unix::process::ExitStatusExt;
            match out.status.signal() {
                Some(sig) => Err(sig),
                None => Ok(out.status.code().unwrap_or(-1)),
            }
        };
        #[cfg(not(unix))]
        let status = Ok(out.status.code().unwrap_or(-1));

        if out.stdout != case.stdout || out.stderr != case.stderr || status != case.status {
            failures.push(format!(
                "stdin = \"{}\"\n  want stdout \"{}\" stderr \"{}\" status {:?}\n   got stdout \"{}\" stderr \"{}\" status {:?}",
                describe(case.stdin),
                describe(case.stdout),
                describe(case.stderr),
                case.status,
                describe(&out.stdout),
                describe(&out.stderr),
                status
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} recorded outcomes differ:\n{}",
        failures.len(),
        CASES.len(),
        failures.join("\n")
    );
}
