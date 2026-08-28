//! Differential tests for the parts that need more than a fixed script:
//! randomised sessions, the wall-clock dependent `time` command, the stdio
//! buffer flush boundaries, and meta-assertions that keep the crash tests from
//! passing vacuously.

mod common;

use common::{
    assert_all, assert_same, c_binary, check, run, run_stdout_to_file, rust_binary, script,
};

// ---------------------------------------------------------------------------
// Meta-assertions
// ---------------------------------------------------------------------------

/// The overrun tests would pass trivially if neither program crashed.  Pin down
/// that the C reference really is killed by `SIGSEGV` on these inputs, and that
/// the Rust program is killed the same way with the same partial output.
#[test]
fn crash_paths_are_really_crash_paths() {
    // 9 users, then a 43-byte password stored into users[9].password.  The
    // overrun leaves user_count == 0x00505050, and the following store to
    // users[user_count].permission_level is far outside the mapping.
    let mut lines: Vec<String> = (0..9).map(|i| format!("adduser u{i} p{i} {i}")).collect();
    lines.push(format!("adduser LAST {} 3", "P".repeat(43)));
    lines.push("status".to_string());
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    let input = script(&refs);

    let c = run(&c_binary(), &input);
    assert_eq!(
        c.signal,
        Some(libc_sigsegv()),
        "expected the C reference to be killed by SIGSEGV on this input, \
         but it {}; the overrun tests would be vacuous",
        describe(&c),
    );

    let r = run(&rust_binary(), &input);
    assert_eq!(
        r.signal,
        Some(libc_sigsegv()),
        "the C reference died by SIGSEGV but the Rust program {}",
        describe(&r),
    );
    assert_eq!(c.stdout, r.stdout, "partial stdout before the fault differs");
    assert_eq!(c.stderr, r.stderr, "stderr before the fault differs");
}

/// Likewise for the normal-exit paths: make sure the harness observes a real
/// exit code and non-empty output, so a silently broken binary cannot pass.
#[test]
fn harness_observes_real_output() {
    let c = run(&c_binary(), &script(&["status"]));
    assert_eq!(c.code, Some(0), "C reference should exit 0 on `status`");
    assert!(c.signal.is_none());
    assert!(
        c.stdout.len() > 100,
        "expected substantial stdout from the C reference, got {} bytes",
        c.stdout.len()
    );
    assert!(
        c.stdout.starts_with(b"|---"),
        "expected the banner at the start of stdout"
    );
    assert!(c.stderr.is_empty(), "the C program never writes to stderr");
}

fn libc_sigsegv() -> i32 {
    11
}

fn describe(r: &common::Run) -> String {
    match (r.code, r.signal) {
        (Some(c), _) => format!("exited with code {c} ({} stdout bytes)", r.stdout.len()),
        (_, Some(s)) => format!("was killed by signal {s} ({} stdout bytes)", r.stdout.len()),
        _ => "had an unknown status".to_string(),
    }
}

// ---------------------------------------------------------------------------
// stdio buffer flush boundaries
// ---------------------------------------------------------------------------

/// When the process dies, everything still sitting in glibc's stdout buffer is
/// lost, so only whole buffer-sized chunks reach the consumer.  Vary how much
/// output is produced before the fault so the truncation point moves across
/// buffer boundaries, and check both a pipe and a regular file (glibc picks the
/// buffer size from `st_blksize`, which can differ between the two).
#[test]
fn buffer_flush_boundaries_on_crash() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for helps in [0usize, 1, 2, 3, 4, 5, 8, 11, 13, 20, 30] {
        let mut lines: Vec<String> = (0..9).map(|i| format!("adduser u{i} p{i} {i}")).collect();
        for _ in 0..helps {
            lines.push("help".to_string());
        }
        lines.push(format!("adduser LAST {} 3", "P".repeat(43)));
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        cases.push((format!("{helps} help(s) before the fault"), script(&refs)));
    }

    // Over a pipe.
    let borrowed: Vec<(&str, Vec<u8>)> =
        cases.iter().map(|(n, i)| (n.as_str(), i.clone())).collect();
    assert_all(&borrowed);

    // And to a regular file.
    let dir = std::env::temp_dir().join(format!("driver-difftest-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("cannot create scratch dir");
    for (name, input) in &cases {
        let cp = dir.join("c.out");
        let rp = dir.join("r.out");
        let (c, c_bytes) = run_stdout_to_file(&c_binary(), input, &cp);
        let (r, r_bytes) = run_stdout_to_file(&rust_binary(), input, &rp);
        assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "[{name}, stdout to a file] exit status differs"
        );
        assert_eq!(
            c_bytes.len(),
            r_bytes.len(),
            "[{name}, stdout to a file] byte count differs: C {} vs Rust {}",
            c_bytes.len(),
            r_bytes.len()
        );
        assert!(
            c_bytes == r_bytes,
            "[{name}, stdout to a file] contents differ"
        );
        assert_eq!(c.stderr, r.stderr, "[{name}, stdout to a file] stderr differs");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The same buffering rules apply on a clean exit, where nothing may be lost.
#[test]
fn buffer_boundaries_on_clean_exit() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for helps in [1usize, 3, 4, 8, 16] {
        let mut lines: Vec<String> = Vec::new();
        for _ in 0..helps {
            lines.push("help".to_string());
        }
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        cases.push((format!("{helps} help(s), EOF"), script(&refs)));

        let mut with_exit: Vec<&str> = refs.clone();
        with_exit.push("exit");
        cases.push((format!("{helps} help(s), exit"), script(&with_exit)));
    }
    let borrowed: Vec<(&str, Vec<u8>)> =
        cases.iter().map(|(n, i)| (n.as_str(), i.clone())).collect();
    assert_all(&borrowed);
}

// ---------------------------------------------------------------------------
// The wall-clock dependent `time` command
// ---------------------------------------------------------------------------

/// `ctime()` renders the current time, so the two subprocesses disagree if a
/// second ticks over between them.  Retry until they line up; the point is that
/// they *can* agree exactly, which they cannot if the format differs.
#[test]
fn time_command_matches_exactly() {
    let input = script(&["time"]);
    let mut last = String::new();
    for _ in 0..40 {
        match check("time", &input) {
            Ok(()) => return,
            Err(e) => last = e,
        }
    }
    panic!("`time` never matched exactly in 40 attempts:\n{last}");
}

/// Clock-independent check of the `time` output: with every ASCII digit masked
/// out, the two programs must agree byte for byte.  This pins the surrounding
/// literal text, the `ctime` layout and the trailing newline (`printf` uses no
/// `\n` of its own, because `ctime` supplies one).
#[test]
fn time_command_format_is_identical() {
    let input = script(&["time", "time", "status"]);
    let c = run(&c_binary(), &input);
    let r = run(&rust_binary(), &input);

    let mask = |b: &[u8]| -> Vec<u8> {
        b.iter()
            .map(|&x| if x.is_ascii_digit() { b'#' } else { x })
            .collect()
    };
    assert_eq!((c.code, c.signal), (r.code, r.signal), "exit status differs");
    assert_eq!(c.stderr, r.stderr, "stderr differs");
    assert_eq!(
        String::from_utf8_lossy(&mask(&c.stdout)),
        String::from_utf8_lossy(&mask(&r.stdout)),
        "digit-masked stdout differs"
    );

    // Sanity-check the shape the C actually produces, so the comparison above
    // is not just two identically wrong programs.
    let text = String::from_utf8_lossy(&c.stdout).to_string();
    assert!(
        text.contains("Current time: "),
        "expected the `Current time: ` prefix in:\n{text}"
    );
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("> Current time: ") {
            // asctime()'s fixed form: "Www Mmm dd hh:mm:ss yyyy".
            assert_eq!(
                rest.len(),
                24,
                "expected a 24 character ctime rendering, got {rest:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Randomised differential testing
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    fn next(&mut self) -> u64 {
        // xorshift64*, deterministic so failures are reproducible.
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len())]
    }
}

const COMMANDS: &[&str] = &[
    "adduser", "login", "logout", "whoami", "listusers", "users", "createfile", "touch",
    "readfile", "cat", "writefile", "write", "deletefile", "rm", "listfiles", "ls", "set", "get",
    "unset", "listvars", "vars", "compare", "cmp", "compareN", "cmpn", "startswith", "match",
    "debug", "verbose", "status", "help", "?", "exit", "quit", "add", "log", "list", "create",
    "read", "delete", "bogus", "", "ADDUSER", "compareN", "?!",
];

const TOKENS: &[&str] = &[
    "a", "b", "abc", "alice", "bob", "pw", "secret", "on", "off", "0", "1", "4", "5", "8", "9",
    "10", "-1", "-5", "2147483647", "-2147483648", "99999999999999999999", "xyz", "f1", "f2",
    "v1", "v2", "hello", "he", "hellothere", "N", "p",
];

const SEPARATORS: &[&str] = &[" ", "  ", "\t", " \t "];

fn random_line(rng: &mut Rng) -> String {
    let mut line = String::new();
    if rng.below(12) == 0 {
        line.push_str(rng.pick(SEPARATORS));
    }
    line.push_str(rng.pick(COMMANDS));
    let argc = rng.below(5);
    for _ in 0..argc {
        line.push_str(rng.pick(SEPARATORS));
        if rng.below(14) == 0 {
            // Occasionally a long token, to exercise truncation and the
            // fixed-buffer copies.
            let n = 20 + rng.below(50);
            line.push_str(&"L".repeat(n));
        } else {
            line.push_str(rng.pick(TOKENS));
        }
    }
    line
}

#[test]
fn randomised_sessions() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for seed in 1..=10u64 {
        let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let lines: Vec<String> = (0..250).map(|_| random_line(&mut rng)).collect();
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        cases.push((format!("random session seed {seed}"), script(&refs)));
    }
    let borrowed: Vec<(&str, Vec<u8>)> =
        cases.iter().map(|(n, i)| (n.as_str(), i.clone())).collect();
    assert_all(&borrowed);
}

/// Random sessions that first fill the user, file and variable tables, so the
/// capacity limits and the end-of-array overruns are reached often.
#[test]
fn randomised_sessions_at_capacity() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for seed in 1..=8u64 {
        let mut rng = Rng::new(seed.wrapping_mul(0xD1B5_4A32_D192_ED03) ^ 0x5DEE_CE66);
        let mut lines: Vec<String> = Vec::new();
        for i in 0..9 {
            lines.push(format!("adduser u{i} p{i} {i}"));
        }
        lines.push("login u0 p0".to_string());
        for i in 0..19 {
            lines.push(format!("createfile g{i} body{i}"));
        }
        for i in 0..19 {
            lines.push(format!("set w{i} val{i}"));
        }
        for _ in 0..120 {
            lines.push(random_line(&mut rng));
        }
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        cases.push((format!("random at-capacity session seed {seed}"), script(&refs)));
    }
    let borrowed: Vec<(&str, Vec<u8>)> =
        cases.iter().map(|(n, i)| (n.as_str(), i.clone())).collect();
    assert_all(&borrowed);
}

/// Random raw bytes, including NULs, CRs and high bytes, to make sure the input
/// path never diverges on malformed data.
#[test]
fn randomised_raw_bytes() {
    let alphabet: &[u8] = b"abc \t\n\r\0\x01\x7f\x80\xff/=:'\"";
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for seed in 1..=8u64 {
        let mut rng = Rng::new(seed.wrapping_mul(0xA076_1D64_78BD_642F) ^ 0xBEEF);
        let n = 400 + rng.below(600);
        let bytes: Vec<u8> = (0..n).map(|_| rng.pick(alphabet)).collect();
        cases.push((format!("random bytes seed {seed}"), bytes));
    }
    let borrowed: Vec<(&str, Vec<u8>)> =
        cases.iter().map(|(n, i)| (n.as_str(), i.clone())).collect();
    assert_all(&borrowed);
}

/// Random command names mixed with random separators only, hammering the
/// dispatch chain (exact matches, aliases, prefix suggestions, unknown).
#[test]
fn randomised_dispatch() {
    let mut rng = Rng::new(0xC0FF_EE12_3456_789B);
    let prefixes = [
        "add", "adduse", "log", "logi", "logou", "list", "listu", "create", "createfil", "read",
        "readfil", "write", "writefil", "delete", "deletefil", "se", "ge", "unse", "compar",
        "startswit", "matc", "debu", "verbos", "statu", "hel", "exi", "qui",
    ];
    let mut lines: Vec<String> = Vec::new();
    for _ in 0..600 {
        let base = if rng.below(2) == 0 {
            rng.pick(&prefixes).to_string()
        } else {
            rng.pick(COMMANDS).to_string()
        };
        let suffix = match rng.below(4) {
            0 => "".to_string(),
            1 => "x".to_string(),
            2 => "file".to_string(),
            _ => "s".to_string(),
        };
        lines.push(format!("{base}{suffix}"));
    }
    // `exit`/`quit` would end the session early; keep them out of this one so
    // the whole script is exercised.
    lines.retain(|l| l != "exit" && l != "quit" && l != "exits" && l != "quits");
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    assert_same("randomised dispatch", &script(&refs));
}
