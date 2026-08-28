//! Differential tests: run the original C program and the Rust translation as
//! subprocesses, feed both the same bytes on stdin and require that stdout,
//! stderr and the exit status are identical.
//!
//! Nothing here links against the translation as a library - the binary is
//! driven exactly the way a shell drives it, because that is how the two
//! programs are compared.

use std::ffi::OsStr;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

/// The translation's binary; cargo builds it for us and hands over the path.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

fn repo_root() -> PathBuf {
    // .../<root>/translation -> .../<root>
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

/// Path of the C `driver`, built with cmake on first use (Phase A build).
fn c_bin() -> PathBuf {
    static BUILT: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    let cell = BUILT.get_or_init(|| Mutex::new(None));
    let mut guard = cell.lock().unwrap();
    if let Some(p) = guard.as_ref() {
        return p.clone();
    }

    let c_src = repo_root().join("c_src");
    let build = c_src.join("build");
    let bin = build.join("driver");

    if !bin.exists() {
        std::fs::create_dir_all(&build).expect("create c_src/build");
        run_tool("cmake", &[OsStr::new("..")], &build);
        run_tool(
            "cmake",
            &[OsStr::new("--build"), OsStr::new(".")],
            &build,
        );
    }
    assert!(
        bin.exists(),
        "the C program was not built at {}",
        bin.display()
    );

    *guard = Some(bin.clone());
    bin
}

fn run_tool(prog: &str, args: &[&OsStr], cwd: &Path) {
    let out = Command::new(prog)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("could not run {prog}: {e}"));
    assert!(
        out.status.success(),
        "{prog} {args:?} failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(n)` for a normal exit, `None` when killed by a signal.
    code: Option<i32>,
    /// `Some(n)` when killed by signal `n` (e.g. 6 = SIGABRT).
    signal: Option<i32>,
}

fn run(bin: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("could not spawn {}: {e}", bin.display()));

    // Feed stdin from another thread so a program that produces more output
    // than a pipe holds can never dead-lock against us.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let bytes = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&bytes);
        let _ = stdin.flush();
        // dropping `stdin` closes it, which is the EOF the program waits for
    });

    let out = child.wait_with_output().expect("wait for child");
    writer.join().expect("stdin writer thread");

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Readable rendering of program output for failure messages.
fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n\n"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// First index at which two byte strings differ, with a little context.
fn first_diff(a: &[u8], b: &[u8]) -> String {
    let at = a
        .iter()
        .zip(b.iter())
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    let from = at.saturating_sub(60);
    format!(
        "first difference at byte {at} (C len {}, Rust len {})\n\
         C    ...{}\n\
         Rust ...{}",
        a.len(),
        b.len(),
        show(&a[from..(at + 60).min(a.len())]),
        show(&b[from..(at + 60).min(b.len())]),
    )
}

/// The whole point: identical stdout, identical stderr, identical exit status.
#[track_caller]
fn assert_identical(name: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(Path::new(RUST_BIN), input);

    assert!(
        c.stdout == r.stdout,
        "[{name}] stdout differs\ninput:\n{}\n{}",
        show(input),
        first_diff(&c.stdout, &r.stdout)
    );
    assert!(
        c.stderr == r.stderr,
        "[{name}] stderr differs\ninput:\n{}\n{}",
        show(input),
        first_diff(&c.stderr, &r.stderr)
    );
    assert!(
        c.code == r.code && c.signal == r.signal,
        "[{name}] exit status differs: C code={:?} signal={:?}, Rust code={:?} signal={:?}\ninput:\n{}",
        c.code,
        c.signal,
        r.code,
        r.signal,
        show(input)
    );
}

/// Run a whole table of cases and report every failure at once.
#[track_caller]
fn assert_all(cases: &[(&str, String)]) {
    let mut failures = Vec::new();
    for (name, input) in cases {
        let c = run(&c_bin(), input.as_bytes());
        let r = run(Path::new(RUST_BIN), input.as_bytes());
        if c.stdout != r.stdout {
            failures.push(format!("[{name}] stdout: {}", first_diff(&c.stdout, &r.stdout)));
        }
        if c.stderr != r.stderr {
            failures.push(format!("[{name}] stderr: {}", first_diff(&c.stderr, &r.stderr)));
        }
        if c.code != r.code || c.signal != r.signal {
            failures.push(format!(
                "[{name}] status: C code={:?} sig={:?} vs Rust code={:?} sig={:?}",
                c.code, c.signal, r.code, r.signal
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} case(s) differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn s(x: &str) -> String {
    x.to_string()
}

// ---------------------------------------------------------------------------
// the menu loop itself: EOF, unparsable choices, out-of-range choices
// ---------------------------------------------------------------------------

#[test]
fn menu_loop_and_choice_parsing() {
    assert_all(&[
        // fgets() returns NULL straight away: menu once, then fall out of the
        // loop and free the graph.
        ("empty input", s("")),
        ("only newline", s("\n")),
        ("only spaces", s("   \n")),
        ("exit immediately", s("8\n")),
        ("exit without trailing newline", s("8")),
        // sscanf("%d") != 1 -> "Invalid input"
        ("letters", s("abc\n8\n")),
        ("punctuation", s("!!\n8\n")),
        ("empty line then exit", s("\n8\n")),
        ("hex looks like 0", s("0x8\n8\n")),
        // parsed, but not a menu entry -> "Invalid choice"
        ("zero", s("0\n8\n")),
        ("nine", s("9\n8\n")),
        ("negative", s("-1\n8\n")),
        // sscanf skips leading whitespace and accepts a sign / trailing junk
        ("leading blanks", s("   3\n8\n")),
        ("leading tab", s("\t8\n")),
        ("plus sign", s("+3\n8\n")),
        ("minus zero", s("-0\n8\n")),
        ("trailing junk", s("3abc\n8\n")),
        ("leading zero", s("08\n")),
        // glibc: strtol saturates at LONG_MAX/LONG_MIN, the store truncates to int
        ("int overflow 2^31", s("2147483648\n8\n")),
        ("wraps to 8", s("4294967304\n8\n")),
        ("long overflow -> -1", s("99999999999999999999\n8\n")),
        ("negative long overflow", s("-99999999999999999999\n8\n")),
        // no menu entry runs after the last line, the loop just ends
        ("choice then EOF", s("3")),
    ]);
}

#[test]
fn long_input_lines_are_split_by_fgets() {
    // fgets() stops after 255 bytes, so the tail of an over-long line is read
    // as the *next* line - which the menu then tries to parse as a choice.
    let long_name = "x".repeat(300);
    let long_choice = format!("8{}", "0".repeat(300));
    assert_all(&[
        ("300 byte city name", format!("1\n{long_name}\n3\n8\n")),
        ("255 byte city name", format!("1\n{}\n3\n8\n", "y".repeat(255))),
        ("256 byte city name", format!("1\n{}\n3\n8\n", "z".repeat(256))),
        ("254 byte city name", format!("1\n{}\n3\n8\n", "w".repeat(254))),
        ("300 byte choice line", format!("{long_choice}\n8\n")),
        ("long line then EOF", format!("1\n{long_name}")),
    ]);
}

#[test]
fn newline_and_nul_handling() {
    // strcspn(input, "\n") stops at the NUL as well, and a lone \r stays part
    // of the city name.
    assert_identical("NUL inside the name", b"1\nAB\x00CD\n3\n4\nAB\n8\n");
    assert_identical("NUL first", b"1\n\x00AB\n3\n8\n");
    assert_identical("CRLF line endings", b"1\r\nA\r\n3\r\n8\r\n");
    assert_identical("no trailing newline", b"1\nA\n3\n8");
    assert_identical("blank city name", b"1\n\n3\n4\n\n8\n");
    // city names are raw bytes, not text: nothing may re-encode them
    assert_identical(
        "invalid UTF-8 in the name",
        b"1\n\xff\xfe\xc3(\n3\n4\n\xff\xfe\xc3(\n5\n\xff\xfe\xc3(\n\xff\xfe\xc3(\n\
          6\n\xff\xfe\xc3(\n7\n\xff\xfe\xc3(\n8\n",
    );
    assert_identical("high bytes only", b"1\n\x80\x81\x82\n1\n\x80\x81\x82\n3\n8\n");
    // leading/trailing blanks are part of the name, strcmp is exact
    assert_identical(
        "blanks are part of the name",
        b"1\n  New York  \n3\n4\n  New York  \n4\nNew York\n8\n",
    );
    // a city called "8" must not be mistaken for a menu choice
    assert_identical("city named 8", b"1\n8\n3\n4\n8\n7\n8\n8\n");
    // sscanf() and strcspn() both stop at the NUL, so the rest of the line is
    // simply invisible
    assert_identical("NUL inside the choice line", b"1\x00abc\nBoston\n3\n8\n");
    assert_identical("line starting with NUL", b"\x008\n8\n");
}

// ---------------------------------------------------------------------------
// case 1: add city
// ---------------------------------------------------------------------------

#[test]
fn add_city() {
    let sixty_three = "1234567890".repeat(6) + "123"; // 63 bytes
    let sixty_four = sixty_three.clone() + "4";
    let seventy = "a".repeat(70);
    assert_all(&[
        ("one city", s("1\nBoston\n3\n8\n")),
        ("two cities", s("1\nBoston\n1\nDenver\n3\n8\n")),
        ("duplicate city", s("1\nA\n1\nA\n3\n8\n")),
        ("duplicate empty name", s("1\n\n1\n\n8\n")),
        ("70 byte name is truncated", format!("1\n{seventy}\n3\n8\n")),
        ("63 byte name fits", format!("1\n{sixty_three}\n3\n8\n")),
        // truncated to the same 63 bytes, but strcmp compares against the full
        // string, so the duplicate check does not fire
        (
            "names equal after truncation",
            format!("1\n{sixty_three}\n1\n{sixty_four}\n3\n8\n"),
        ),
        // EOF at the "Enter city name:" prompt
        ("EOF after choice 1", s("1\n")),
        ("EOF after choice 1, no newline", s("1")),
        ("name without newline at EOF", s("1\nBoston")),
    ]);
}

#[test]
fn graph_is_full_at_max_nodes() {
    let mut input = String::new();
    for i in 1..=101 {
        input.push_str(&format!("1\nN{i}\n"));
    }
    input.push_str("3\n8\n");
    assert_identical("101 cities", input.as_bytes());

    // add_node() tests "graph is full" *before* the duplicate check, so at
    // capacity even a duplicate name is refused with "Graph is full".
    let mut input = String::new();
    for i in 1..=100 {
        input.push_str(&format!("1\nN{i}\n"));
    }
    input.push_str("1\nN1\n1\nZZ\n3\n8\n");
    assert_identical("duplicate name at capacity", input.as_bytes());
}

// ---------------------------------------------------------------------------
// case 2: add route
// ---------------------------------------------------------------------------

#[test]
fn add_route() {
    assert_all(&[
        ("both cities missing", s("2\nA\nB\n5\n8\n")),
        ("to city missing", s("1\nA\n2\nA\nB\n5\n8\n")),
        ("from city missing", s("1\nB\n2\nA\nB\n5\n8\n")),
        ("route added", s("1\nA\n1\nB\n2\nA\nB\n5\n3\n8\n")),
        ("distance zero", s("1\nA\n1\nB\n2\nA\nB\n0\n3\n8\n")),
        ("unparsable distance", s("1\nA\n1\nB\n2\nA\nB\nxyz\n8\n")),
        ("empty distance line", s("1\nA\n1\nB\n2\nA\nB\n\n8\n")),
        ("distance with junk", s("1\nA\n1\nB\n2\nA\nB\n5xyz\n3\n8\n")),
        ("negative distance", s("1\nA\n1\nB\n2\nA\nB\n-5\n8\n")),
        // truncation makes these negative / positive again
        ("distance 2^31", s("1\nA\n1\nB\n2\nA\nB\n2147483648\n3\n8\n")),
        ("distance 2^32+5", s("1\nA\n1\nB\n2\nA\nB\n4294967301\n3\n8\n")),
        ("distance long overflow", s("1\nA\n1\nB\n2\nA\nB\n99999999999999999999\n3\n8\n")),
        (
            "distance negative overflow",
            s("1\nA\n1\nB\n2\nA\nB\n-99999999999999999999\n3\n8\n"),
        ),
        ("distance INT_MAX", s("1\nA\n1\nB\n2\nA\nB\n2147483647\n3\n8\n")),
        ("duplicate edge", s("1\nA\n1\nB\n2\nA\nB\n5\n2\nA\nB\n7\n3\n8\n")),
        ("self edge", s("1\nA\n2\nA\nA\n3\n4\nA\n8\n")),
        ("duplicate self edge", s("1\nA\n2\nA\nA\n1\n2\nA\nA\n2\n3\n8\n")),
        // main() parses the distance before it looks the cities up, so an
        // unparsable distance wins over a missing city
        ("missing cities and a bad distance", s("2\nX\nY\nabc\n8\n")),
        // ... but a negative one is only caught inside add_edge, after the
        // lookups, so there the missing city wins
        ("missing city and a negative distance", s("2\nX\nY\n-1\n8\n")),
        // EOF at each of the three prompts
        ("EOF at from prompt", s("2\n")),
        ("EOF at to prompt", s("2\nA\n")),
        ("EOF at distance prompt", s("2\nA\nB\n")),
    ]);
}

#[test]
fn node_has_maximum_edges() {
    let mut input = String::from("1\nA\n");
    for i in 1..=11 {
        input.push_str(&format!("1\nC{i}\n"));
    }
    for i in 1..=11 {
        input.push_str(&format!("2\nA\nC{i}\n{i}\n"));
    }
    input.push_str("4\nA\n8\n");
    assert_identical("11 edges on one node", input.as_bytes());

    // add_edge() checks the edge count first, the distance second and the
    // duplicate last, so a full node reports "maximum edges" even for a
    // negative distance, and a duplicate edge with a negative distance
    // reports the distance.
    let mut input = String::from("1\nA\n");
    for i in 1..=11 {
        input.push_str(&format!("1\nC{i}\n"));
    }
    for i in 1..=10 {
        input.push_str(&format!("2\nA\nC{i}\n{i}\n"));
    }
    input.push_str("2\nA\nC11\n-5\n2\nA\nC1\n-5\n4\nA\n8\n");
    assert_identical("validation order in add_edge", input.as_bytes());
}

// ---------------------------------------------------------------------------
// cases 3 and 4: show all cities / show city details
// ---------------------------------------------------------------------------

#[test]
fn show_cities_and_details() {
    let mut big = String::new();
    for i in 1..=40 {
        big.push_str(&format!("1\nCity{i}\n"));
    }
    big.push_str("3\n8\n");
    assert_all(&[
        ("show empty graph", s("3\n8\n")),
        ("show twice", s("1\nA\n3\n3\n8\n")),
        ("details of missing city", s("4\nNope\n8\n")),
        ("details of city without edges", s("1\nA\n4\nA\n8\n")),
        ("details of city with edges", s("1\nA\n1\nB\n2\nA\nB\n5\n4\nA\n4\nB\n8\n")),
        ("EOF at details prompt", s("4\n")),
        // more than one 4096 byte stdout block, so the write boundaries matter
        ("40 cities", big),
    ]);
}

// ---------------------------------------------------------------------------
// case 5: shortest path
// ---------------------------------------------------------------------------

#[test]
fn shortest_path() {
    assert_all(&[
        ("start missing", s("5\nX\nY\n8\n")),
        ("end missing", s("1\nA\n5\nA\nY\n8\n")),
        ("start is end", s("1\nA\n5\nA\nA\n8\n")),
        ("no edges at all", s("1\nA\n1\nB\n5\nA\nB\n8\n")),
        ("edge only backwards", s("1\nA\n1\nB\n2\nB\nA\n5\n5\nA\nB\n8\n")),
        ("single edge", s("1\nA\n1\nB\n2\nA\nB\n5\n5\nA\nB\n8\n")),
        (
            "two hops beat the direct edge",
            s("1\nA\n1\nB\n1\nC\n2\nA\nB\n5\n2\nB\nC\n3\n2\nA\nC\n20\n5\nA\nC\n8\n"),
        ),
        (
            "direct edge beats two hops",
            s("1\nA\n1\nB\n1\nC\n2\nA\nB\n5\n2\nB\nC\n3\n2\nA\nC\n2\n5\nA\nC\n8\n"),
        ),
        (
            "zero weight edges",
            s("1\nA\n1\nB\n1\nC\n2\nA\nB\n0\n2\nB\nC\n0\n5\nA\nC\n8\n"),
        ),
        // an unreachable third city keeps its INT_MAX distance
        (
            "unreachable city",
            s("1\nA\n1\nB\n1\nC\n2\nA\nB\n1\n5\nA\nC\n8\n"),
        ),
        // INT_MAX is exactly the "infinity" the algorithm uses, so the
        // neighbour never gets updated and no path is found
        (
            "edge of length INT_MAX",
            s("1\nA\n1\nB\n2\nA\nB\n2147483647\n5\nA\nB\n8\n"),
        ),
        // 2147483646 + 2147483646 overflows int
        (
            "distance sum overflows",
            s("1\nA\n1\nB\n1\nC\n2\nA\nB\n2147483646\n2\nB\nC\n2147483646\n5\nA\nC\n8\n"),
        ),
        (
            "cycle between two cities",
            s("1\nA\n1\nB\n2\nA\nB\n1\n2\nB\nA\n1\n5\nA\nB\n5\nB\nA\n8\n"),
        ),
        (
            "self loop",
            s("1\nA\n1\nB\n2\nA\nA\n1\n2\nA\nB\n2\n5\nA\nB\n8\n"),
        ),
        (
            "diamond",
            s("1\nA\n1\nB\n1\nC\n1\nD\n2\nA\nB\n1\n2\nA\nC\n1\n2\nB\nD\n5\n2\nC\nD\n2\n5\nA\nD\n8\n"),
        ),
        ("EOF at start prompt", s("5\n")),
        ("EOF at end prompt", s("5\nA\n")),
    ]);
}

#[test]
fn shortest_path_long_chain() {
    // a 30 hop chain: exercises the path reconstruction and its malloc
    let mut input = String::new();
    for i in 1..=30 {
        input.push_str(&format!("1\nC{i}\n"));
    }
    for i in 1..30 {
        input.push_str(&format!("2\nC{i}\nC{}\n{i}\n", i + 1));
    }
    input.push_str("5\nC1\nC30\n5\nC30\nC1\n8\n");
    assert_identical("30 city chain", input.as_bytes());
}

#[test]
fn hundred_node_graph() {
    // MAX_NODES cities in a chain: the Dijkstra state array and the
    // reconstructed path both reach exactly MAX_NODES entries.
    let mut input = String::new();
    for i in 1..=100 {
        input.push_str(&format!("1\nC{i}\n"));
    }
    for i in 1..100 {
        input.push_str(&format!("2\nC{i}\nC{}\n1\n", i + 1));
    }
    let chain = input.clone() + "5\nC1\nC100\n6\nC1\n3\n8\n";
    assert_identical("100 city chain", chain.as_bytes());

    // ... and the same with an edge closing the cycle
    let cyclic = input + "2\nC100\nC1\n1\n5\nC1\nC100\n5\nC50\nC49\n6\nC1\n8\n";
    assert_identical("100 city cycle", cyclic.as_bytes());
}

/// A chain of `k` cities plus one back-edge whose weight makes the distance sum
/// overflow.  The back-edge sits on the second-to-last city, so it is explored
/// before the loop reaches the end city, and it lowers the (already final)
/// distance of city `j` - which rewrites `previous` and makes the chain cyclic.
fn overflow_back_edge(k: usize, j: usize) -> String {
    let mut input = String::new();
    for i in 1..=k {
        input.push_str(&format!("1\nC{i}\n"));
    }
    for i in 1..k {
        input.push_str(&format!("2\nC{i}\nC{}\n1\n", i + 1));
    }
    input.push_str(&format!("2\nC{}\nC{j}\n2147483647\n", k - 1));
    input.push_str(&format!("5\nC1\nC{k}\n8\n"));
    input
}

/// Reconstructing such a cyclic chain runs past the end of
/// `node_t *path[MAX_NODES]`, and gcc puts `path` directly below the `state`
/// array: the writes land in `state`, which the loop itself reads back.  Most
/// of the time that stops the loop - `state[0].node` becomes the current node,
/// so the search finds index 0 and takes `state[0].previous`, which is NULL for
/// the start node - and the C program happily prints a 101 entry path and exits
/// 0.  It must not be mistaken for a crash.
#[test]
fn path_reconstruction_overruns_into_the_dijkstra_state() {
    // the plain 101 entry case, spelled out
    assert_identical(
        "101 entry path",
        b"1\nA\n1\nB\n1\nC\n1\nD\n2\nA\nB\n1\n2\nB\nC\n1\n2\nC\nB\n2147483647\n\
          2\nC\nD\n1\n5\nA\nD\n8\n",
    );
    // Back-edges into the start city keep the chain going for a few more words,
    // which land in state[0].distance, state[0].previous, state[0].visited and
    // then in state[1]... - 107, 111 and 110 entries respectively.
    for (k, j) in [(4, 2), (5, 1), (5, 2), (6, 1), (7, 1), (8, 2), (10, 2)] {
        assert_identical(
            &format!("chain of {k} with a back-edge to C{j}"),
            overflow_back_edge(k, j).as_bytes(),
        );
    }
}

/// The same overrun, but for these graphs the writes keep going until they are
/// past the `state` array as well and the frame is destroyed: the C program dies
/// from a signal without printing anything more.
///
/// Which signal it is (SIGSEGV or, less often, SIGBUS) depends on what the
/// runaway writes clobber first and varies between runs of the *C* program, so
/// the assertion is "both die from a signal, with identical output".
#[test]
fn distance_overflow_makes_the_previous_chain_cyclic() {
    let cycle = "1\nA\n1\nB\n1\nC\n2\nA\nB\n2000000000\n2\nB\nC\n2000000000\n\
                 2\nB\nA\n2000000000\n5\nA\nC\n8\n";

    for (name, input) in [
        ("crash with an empty stdout buffer", cycle.to_string()),
        // enough output beforehand that whole 4096 byte blocks have already
        // been written when the process dies
        ("crash after one full stdout block", {
            let mut s = String::new();
            for i in 1..=10 {
                s.push_str(&format!("1\nP{i}\n"));
            }
            s + cycle
        }),
        ("crash after two full stdout blocks", {
            let mut s = String::new();
            for i in 1..=40 {
                s.push_str(&format!("1\nP{i}\n"));
            }
            s + cycle
        }),
        // chains whose overrun does not settle down inside `state` either
        ("3 city chain, back-edge to the start", overflow_back_edge(3, 1)),
        ("4 city chain, back-edge to the start", overflow_back_edge(4, 1)),
        ("8 city chain, back-edge to the start", overflow_back_edge(8, 1)),
        ("10 city chain, back-edge to the start", overflow_back_edge(10, 1)),
    ] {
        let r = run(Path::new(RUST_BIN), input.as_bytes());
        for _ in 0..3 {
            let c = run(&c_bin(), input.as_bytes());
            assert_eq!(
                show(&c.stdout),
                show(&r.stdout),
                "[{name}] stdout must match"
            );
            assert_eq!(
                show(&c.stderr),
                show(&r.stderr),
                "[{name}] stderr must match"
            );
            assert_eq!(c.code, None, "[{name}] the C program must die by signal");
            assert_eq!(r.code, None, "[{name}] the Rust program must die by signal");
            // 11 = SIGSEGV, 7 = SIGBUS
            assert!(
                matches!(c.signal, Some(11) | Some(7)),
                "[{name}] unexpected C signal {:?}",
                c.signal
            );
            assert_eq!(r.signal, Some(11), "[{name}] Rust must die from SIGSEGV");
        }
    }
}

// ---------------------------------------------------------------------------
// case 6: shallow copy
// ---------------------------------------------------------------------------

#[test]
fn shallow_copy() {
    assert_all(&[
        ("city missing", s("6\nZ\n8\n")),
        ("single city", s("1\nA\n6\nA\n4\nA\n8\n")),
        ("twice on the same city", s("1\nA\n6\nA\n6\nA\n8\n")),
        (
            "chain of three",
            s("1\nA\n1\nB\n1\nC\n2\nA\nB\n1\n2\nB\nC\n1\n6\nA\n3\n8\n"),
        ),
        // increment_refs_recursive() must not loop forever on a cycle
        (
            "cycle",
            s("1\nA\n1\nB\n2\nA\nB\n1\n2\nB\nA\n1\n6\nA\n3\n8\n"),
        ),
        (
            "self loop",
            s("1\nA\n2\nA\nA\n1\n6\nA\n3\n8\n"),
        ),
        (
            "diamond counts a shared node once",
            s("1\nA\n1\nB\n1\nC\n1\nD\n2\nA\nB\n1\n2\nA\nC\n1\n2\nB\nD\n1\n2\nC\nD\n1\n6\nA\n3\n8\n"),
        ),
        ("copy of a leaf", s("1\nA\n1\nB\n2\nA\nB\n1\n6\nB\n3\n8\n")),
        ("EOF at prompt", s("6\n")),
    ]);
}

// ---------------------------------------------------------------------------
// case 7: delete node - reference counting and the freed-chunk behaviour
// ---------------------------------------------------------------------------

#[test]
fn delete_node_reference_counting() {
    assert_all(&[
        ("city missing", s("7\nZ\n8\n")),
        ("EOF at prompt", s("7\n")),
        // ref_count 2 -> 1: nothing is freed, the node stays usable
        ("after a shallow copy", s("1\nA\n6\nA\n7\nA\n3\n4\nA\n8\n")),
        ("twice after two copies", s("1\nA\n6\nA\n6\nA\n7\nA\n7\nA\n3\n8\n")),
        // ref_count 1 -> 0: free(), and the graph keeps the dangling pointer
        ("delete then exit", s("1\nBoston\n7\nBoston\n8\n")),
        ("delete then EOF", s("1\nBoston\n7\nBoston\n")),
        // free() overwrites the start of city_name, so strcmp() no longer
        // matches the name: every later lookup reports "not found"
        ("delete twice", s("1\nA\n7\nA\n7\nA\n8\n")),
        ("details after delete", s("1\nA\n7\nA\n4\nA\n8\n")),
        ("route from a deleted city", s("1\nA\n1\nB\n7\nB\n2\nA\nB\n5\n8\n")),
        ("route into a deleted city", s("1\nA\n1\nB\n7\nA\n2\nA\nB\n5\n8\n")),
        ("path from a deleted city", s("1\nA\n1\nB\n2\nA\nB\n5\n7\nB\n5\nA\nB\n8\n")),
        ("shallow copy of a deleted city", s("1\nA\n7\nA\n6\nA\n8\n")),
        // the duplicate check no longer matches either, so the name can be
        // added a second time - and malloc hands back the very same chunk
        ("re-add after delete", s("1\nA\n7\nA\n1\nA\n3\n8\n")),
        ("re-add under a new name", s("1\nA\n7\nA\n1\nB\n3\n4\nB\n8\n")),
    ]);
}

#[test]
fn freed_chunks_are_reused_most_recent_first() {
    // The graph keeps the old pointers, so print_graph() shows which chunk each
    // new node landed in: this pins down malloc's reuse order.
    assert_all(&[
        ("two freed, two added", s("1\nA\n1\nB\n7\nA\n7\nB\n1\nC\n1\nD\n3\n8\n")),
        (
            "interleaved free and alloc",
            s("1\nN1\n1\nN2\n1\nN3\n7\nN2\n1\nX\n7\nN1\n7\nN3\n1\nY\n1\nZ\n3\n8\n"),
        ),
        ("free and re-add three times", s("1\nA\n7\nA\n1\nA\n7\nA\n1\nA\n3\n8\n")),
    ]);
}

#[test]
fn freed_chunks_beyond_the_tcache_limit() {
    // More than 7 chunks of the same size are freed, so the tcache overflows
    // and the rest come back in a different order.
    for (name, order) in [
        ("ascending", (1..=9).collect::<Vec<_>>()),
        ("descending", (1..=9).rev().collect::<Vec<_>>()),
    ] {
        let mut input = String::new();
        for i in 1..=9 {
            input.push_str(&format!("1\nN{i}\n"));
        }
        for i in &order {
            input.push_str(&format!("7\nN{i}\n"));
        }
        for i in 1..=9 {
            input.push_str(&format!("1\nM{i}\n"));
        }
        input.push_str("3\n8\n");
        assert_identical(&format!("9 nodes freed {name}"), input.as_bytes());
    }

    let mut input = String::new();
    for i in 1..=16 {
        input.push_str(&format!("1\nN{i}\n"));
    }
    for i in 1..=16 {
        input.push_str(&format!("7\nN{i}\n"));
    }
    for i in 1..=16 {
        input.push_str(&format!("1\nM{i}\n"));
    }
    input.push_str("3\n8\n");
    assert_identical("16 nodes freed and re-added", input.as_bytes());
}

/// With a full tcache bin the freed chunks go to the unsorted bin, and the next
/// `malloc` empties that bin into the tcache oldest first before popping it -
/// so the *newest* of those chunks is handed out first, whatever their
/// addresses are.  Both orders are checked, because "newest first" and "lowest
/// address first" only differ here.
#[test]
fn overflowing_the_tcache_hands_chunks_back_newest_first() {
    for (name, first, second) in [("high then low", 11, 9), ("low then high", 9, 11)] {
        let mut input = String::new();
        for i in 1..=12 {
            input.push_str(&format!("1\nN{i}\n"));
        }
        for i in 1..=7 {
            input.push_str(&format!("7\nN{i}\n")); // fills the tcache bin
        }
        // both of these are isolated: their neighbours are still allocated
        input.push_str(&format!("7\nN{first}\n7\nN{second}\n"));
        for i in 1..=9 {
            input.push_str(&format!("1\nM{i}\n"));
        }
        input.push_str("3\n8\n");
        assert_identical(&format!("isolated overflow chunks, {name}"), input.as_bytes());
    }
}

/// `free()` only writes its metadata into the chunk that actually ends up on a
/// list, so some freed nodes keep a readable `city_name` and stay findable.
#[test]
fn some_freed_nodes_keep_their_name() {
    // The highest chunk borders the top chunk and is absorbed into it, which
    // rewrites only the chunk header - N10 is still found by name afterwards,
    // while the tcache'd N1 is not.
    let mut input = String::new();
    for i in 1..=10 {
        input.push_str(&format!("1\nN{i}\n"));
    }
    for i in 1..=7 {
        input.push_str(&format!("7\nN{i}\n"));
    }
    input.push_str("7\nN10\n4\nN10\n4\nN1\n8\n");
    assert_identical("chunk absorbed into the top chunk", input.as_bytes());

    // N9 is merged into the run that starts at N8, so `fd`/`bk` land in N8 and
    // N9 keeps its name.
    let mut input = String::new();
    for i in 1..=10 {
        input.push_str(&format!("1\nN{i}\n"));
    }
    for i in 1..=7 {
        input.push_str(&format!("7\nN{i}\n"));
    }
    input.push_str("7\nN8\n7\nN9\n4\nN9\n4\nN8\n8\n");
    assert_identical("chunk merged into a free run", input.as_bytes());
}

#[test]
fn freed_chunks_and_the_shortest_path_allocation() {
    // find_shortest_path() mallocs `count * 8` bytes for the result, which for a
    // 30 node path is the same malloc size class as node_t itself - so the path
    // can take a chunk off the node free list and give it back.  Both the
    // "tcache already drained" and the "tcache still full" case must leave the
    // reuse order of the following add_city calls alone.
    let mut chain = String::new();
    for i in 1..=30 {
        chain.push_str(&format!("1\nC{i}\n"));
    }
    for i in 1..30 {
        chain.push_str(&format!("2\nC{i}\nC{}\n1\n", i + 1));
    }
    // nine freed chunks: seven in the tcache, two on the overflow list
    let mut freed = String::new();
    for i in 1..=9 {
        freed.push_str(&format!("1\nD{i}\n"));
    }
    for i in 1..=9 {
        freed.push_str(&format!("7\nD{i}\n"));
    }

    let mut drained = chain.clone() + &freed;
    for i in 1..=7 {
        drained.push_str(&format!("1\nE{i}\n"));
    }
    drained.push_str("5\nC1\nC30\n1\nF1\n1\nF2\n3\n8\n");
    assert_identical("path allocation with a drained tcache", drained.as_bytes());

    let mut full = chain + &freed + "5\nC1\nC30\n";
    for i in 1..=9 {
        full.push_str(&format!("1\nF{i}\n"));
    }
    full.push_str("3\n8\n");
    assert_identical("path allocation with a full tcache", full.as_bytes());
}

// ---------------------------------------------------------------------------
// the two ways this program manages to free the same chunk twice: glibc prints
// a diagnostic and aborts, which kills the buffered stdout with it
// ---------------------------------------------------------------------------

#[test]
fn double_free_aborts_the_process() {
    // shallow_copy() walks the edges and revives the already freed B
    // (ref_count 0 -> 1); free_graph() then frees it a second time.
    assert_identical(
        "double free of a tcache chunk",
        b"1\nA\n1\nB\n2\nA\nB\n5\n7\nB\n6\nA\n8\n",
    );
    assert_identical(
        "double free reached at EOF",
        b"1\nA\n1\nB\n2\nA\nB\n5\n7\nB\n6\nA\n",
    );

    // The same, but the doubly freed chunk did not fit in the tcache, which
    // glibc reports differently.  There is enough output here that the first
    // 4096 byte block has already been written when the process dies.
    let mut input = String::from("1\nH\n");
    for i in 1..=9 {
        input.push_str(&format!("1\nN{i}\n"));
    }
    input.push_str("2\nH\nN9\n5\n");
    for i in 1..=7 {
        input.push_str(&format!("7\nN{i}\n"));
    }
    // N9 is the highest chunk, so it is absorbed into the top chunk
    input.push_str("7\nN9\n6\nH\n8\n");
    assert_identical("double free of a chunk in the top chunk", input.as_bytes());

    // And once more with the doubly freed chunk on a bin instead (N10 stays
    // allocated above it), which glibc reports differently again.
    let mut input = String::from("1\nH\n");
    for i in 1..=10 {
        input.push_str(&format!("1\nN{i}\n"));
    }
    input.push_str("2\nH\nN9\n5\n");
    for i in 1..=7 {
        input.push_str(&format!("7\nN{i}\n"));
    }
    input.push_str("7\nN9\n6\nH\n8\n");
    assert_identical("double free of a binned chunk", input.as_bytes());
}

// ---------------------------------------------------------------------------
// longer sessions that mix everything
// ---------------------------------------------------------------------------

#[test]
fn mixed_sessions() {
    assert_all(&[
        (
            "full walk through the menu",
            s("1\nBoston\n1\nDenver\n1\nAustin\n2\nBoston\nDenver\n100\n\
               2\nDenver\nAustin\n200\n2\nBoston\nAustin\n500\n3\n4\nBoston\n\
               5\nBoston\nAustin\n6\nBoston\n3\n7\nDenver\n3\n8\n"),
        ),
        (
            "errors everywhere",
            s("nope\n0\n99\n2\nA\nB\n1\n4\nA\n5\nA\nB\n6\nA\n7\nA\n1\nA\n1\nA\n\
               2\nA\nA\n-1\n3\n8\n"),
        ),
        (
            "no exit command, just EOF",
            s("1\nA\n1\nB\n2\nA\nB\n7\n3\n"),
        ),
        (
            "menu after every operation",
            s("3\n1\nA\n3\n4\nA\n3\n6\nA\n3\n7\nA\n3\n8\n"),
        ),
    ]);
}

// ---------------------------------------------------------------------------
// the one input class whose C output is not reproducible: printing a node
// whose chunk is on a free list.  city_name then holds glibc's free-list
// metadata, i.e. a mangled heap pointer, so the C program prints different
// bytes on every run.  This test proves that (rather than quietly skipping the
// case) and still checks everything around it.
// ---------------------------------------------------------------------------

#[test]
fn printing_a_freed_node_is_not_reproducible_in_c() {
    let input = b"1\nA\n7\nA\n3\n8\n";
    let runs: Vec<Run> = (0..6).map(|_| run(&c_bin(), input)).collect();

    // stderr and the exit status are stable, so they must match exactly.
    let r = run(Path::new(RUST_BIN), input);
    for c in &runs {
        assert_eq!(c.stderr, r.stderr, "stderr must match");
        assert_eq!((c.code, c.signal), (r.code, r.signal), "status must match");
    }

    // The C program disagrees with itself about the freed node's name, so no
    // translation can match those bytes.  Everything up to the name does match.
    let differ = runs.iter().any(|c| c.stdout != runs[0].stdout);
    let aslr = std::fs::read_to_string("/proc/sys/kernel/randomize_va_space")
        .map(|s| s.trim() != "0")
        .unwrap_or(true);
    assert!(
        differ || !aslr,
        "ASLR is on, yet the C program printed the freed node identically {} \
         times - if the heap layout really is reproducible then the Rust output \
         has to match those bytes, which it cannot, so this case needs \
         revisiting",
        runs.len()
    );

    let prefix = b"City: ";
    let at = runs[0]
        .stdout
        .windows(prefix.len())
        .position(|w| w == prefix)
        .expect("the freed node is printed");
    for c in &runs {
        assert_eq!(
            &c.stdout[..at + prefix.len()],
            &r.stdout[..at + prefix.len()],
            "output before the freed node's name must match"
        );
    }
    // ... and the ref_count printed after the garbled name is still the 0 that
    // free() left behind, in both programs.
    assert!(
        r.stdout.ends_with(
            b" (ref_count: 0)\n  Edges:\n\n=== DAG City Route Manager ===\n\
1. Add city (node)\n2. Add route (edge)\n3. Show all cities\n4. Show city details\n\
5. Find shortest path\n6. Make shallow copy of subsection\n7. Delete node\n8. Exit\n\
Choice: Freeing graph and exiting...\n"
        ),
        "unexpected Rust tail:\n{}",
        show(&r.stdout)
    );
    for c in &runs {
        assert!(
            c.stdout.ends_with(b" (ref_count: 0)\n  Edges:\n\n=== DAG City Route Manager ===\n\
1. Add city (node)\n2. Add route (edge)\n3. Show all cities\n4. Show city details\n\
5. Find shortest path\n6. Make shallow copy of subsection\n7. Delete node\n8. Exit\n\
Choice: Freeing graph and exiting...\n"),
            "unexpected C tail:\n{}",
            show(&c.stdout)
        );
    }
}
