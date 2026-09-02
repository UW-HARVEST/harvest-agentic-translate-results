//! Randomized differential testing with fixed seeds, so every run covers the
//! same inputs.
//!
//! Some inputs make the C program read a city name out of a chunk the allocator
//! has already overwritten with a heap address.  Those bytes move with ASLR, so
//! the C program is not reproducible for them and no translation can be byte
//! identical to it.  Rather than dropping those inputs, this file detects them
//! by running the C program twice and then holds the Rust program to everything
//! that *is* reproducible: stderr and the exit status.  See `ERRORS.md`.

mod harness;

use harness::{run_c, run_rust, Outcome};

/// xorshift64*, so the inputs are identical on every machine and every run.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

/// A general walk over the menu, including the malformed answers.
fn gen_general(rng: &mut Rng) -> Vec<u8> {
    let cities = ["A", "B", "C", "D", "Zurich", "", "a b", "XXXXXXXX"];
    let distances = ["1", "0", "-1", "10", "2147483647", "zz", ""];
    let choices = [1usize, 1, 1, 2, 2, 3, 4, 5, 6, 7, 7, 8, 9, 0];
    let mut lines: Vec<String> = Vec::new();
    let n = rng.range(1, 40);
    for _ in 0..n {
        let c = *rng.pick(&choices);
        lines.push(c.to_string());
        match c {
            1 | 4 | 6 | 7 => lines.push(rng.pick(&cities).to_string()),
            2 => {
                lines.push(rng.pick(&cities).to_string());
                lines.push(rng.pick(&cities).to_string());
                lines.push(rng.pick(&distances).to_string());
            }
            5 => {
                lines.push(rng.pick(&cities).to_string());
                lines.push(rng.pick(&cities).to_string());
            }
            8 => break,
            _ => {}
        }
    }
    let mut s = lines.join("\n");
    s.push('\n');
    s.into_bytes()
}

/// Add a batch of cities and edges, delete a random subset, then add exactly as
/// many fresh cities so that every freed chunk is handed back out.  The chunk
/// reuse order is then fully visible in the final `print_graph`.
fn gen_reuse(rng: &mut Rng) -> Vec<u8> {
    let n = rng.range(1, 40);
    let old: Vec<String> = (0..n).map(|i| format!("O{i}")).collect();
    let mut lines: Vec<String> = Vec::new();
    for c in &old {
        lines.push("1".into());
        lines.push(c.clone());
    }
    for _ in 0..rng.below(n + 1) {
        lines.push("2".into());
        lines.push(rng.pick(&old).clone());
        lines.push(rng.pick(&old).clone());
        lines.push(rng.below(10).to_string());
    }
    // A random subset, in a random order.
    let mut victims: Vec<String> = old.clone();
    for i in (1..victims.len()).rev() {
        let j = rng.below(i + 1);
        victims.swap(i, j);
    }
    victims.truncate(rng.below(n + 1));
    for v in &victims {
        lines.push("7".into());
        lines.push(v.clone());
    }
    for i in 0..victims.len() {
        lines.push("1".into());
        lines.push(format!("N{i}"));
    }
    lines.push("3".into());
    lines.push("8".into());
    let mut s = lines.join("\n");
    s.push('\n');
    s.into_bytes()
}

/// Drive the reference counting hard enough that the heap consistency checks
/// fire: shallow copies revive freed chunks and `free_graph` frees them again.
fn gen_abort(rng: &mut Rng) -> Vec<u8> {
    let n = rng.range(1, 14);
    let names: Vec<String> = (0..n).map(|i| format!("C{i}")).collect();
    let mut lines: Vec<String> = Vec::new();
    for c in &names {
        lines.push("1".into());
        lines.push(c.clone());
    }
    for _ in 0..rng.below(3 * n + 1) {
        lines.push("2".into());
        lines.push(rng.pick(&names).clone());
        lines.push(rng.pick(&names).clone());
        lines.push(rng.below(6).to_string());
    }
    for _ in 0..rng.range(1, 3 * n) {
        lines.push(rng.pick(&[6usize, 7, 7, 7, 4]).to_string());
        lines.push(rng.pick(&names).clone());
    }
    lines.push("8".into());
    let mut s = lines.join("\n");
    s.push('\n');
    s.into_bytes()
}

struct Report {
    checked: usize,
    aslr_dependent: usize,
    aborted: usize,
}

fn sweep(name: &str, gen: fn(&mut Rng) -> Vec<u8>, seeds: u64) -> Report {
    let mut report = Report {
        checked: 0,
        aslr_dependent: 0,
        aborted: 0,
    };
    for seed in 0..seeds {
        let input = gen(&mut Rng::new(seed));
        let first = run_c(&input);
        let second = run_c(&input);
        let rust = run_rust(&input);

        // stderr and the exit status are reproducible for every input.
        assert_eq!(
            (&first.stderr, &first.status),
            (&second.stderr, &second.status),
            "{name} seed {seed}: the C program's own stderr/status is not \
             reproducible, which this harness assumes\ninput: {:?}",
            String::from_utf8_lossy(&input)
        );
        assert_stream(name, seed, &input, "stderr", &first.stderr, &rust.stderr);
        assert!(
            first.status == rust.status,
            "{name} seed {seed}: exit status differs: C {:?}, Rust {:?}\ninput: {:?}",
            first.status,
            rust.status,
            String::from_utf8_lossy(&input)
        );

        if matches!(first.status, Err(_)) {
            report.aborted += 1;
        }

        if first.stdout == second.stdout {
            assert_stream(name, seed, &input, "stdout", &first.stdout, &rust.stdout);
            report.checked += 1;
        } else {
            // The C program printed bytes the allocator derived from a heap
            // address; see ERRORS.md.
            report.aslr_dependent += 1;
        }
    }
    report
}

#[track_caller]
fn assert_stream(name: &str, seed: u64, input: &[u8], which: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    let at = c
        .iter()
        .zip(r.iter())
        .position(|(x, y)| x != y)
        .unwrap_or(c.len().min(r.len()));
    let from = at.saturating_sub(80);
    panic!(
        "{name} seed {seed}: {which} differs at byte {at} (C {} bytes, Rust {} bytes)\n\
         C   ...{:?}\nRust...{:?}\ninput: {:?}",
        c.len(),
        r.len(),
        String::from_utf8_lossy(&c[from..(at + 80).min(c.len())]),
        String::from_utf8_lossy(&r[from..(at + 80).min(r.len())]),
        String::from_utf8_lossy(&input[..input.len().min(4000)]),
    );
}

#[test]
fn random_menu_walks() {
    let r = sweep("general", gen_general, 250);
    assert!(r.checked > 200, "too few fully comparable inputs: {}", r.checked);
    eprintln!(
        "general: {} inputs compared byte for byte, {} ASLR dependent in C",
        r.checked, r.aslr_dependent
    );
}

#[test]
fn random_chunk_reuse() {
    let r = sweep("reuse", gen_reuse, 200);
    assert!(r.checked > 150, "too few fully comparable inputs: {}", r.checked);
    eprintln!(
        "reuse: {} inputs compared byte for byte, {} ASLR dependent in C",
        r.checked, r.aslr_dependent
    );
}

#[test]
fn random_use_after_free() {
    let r = sweep("abort", gen_abort, 80);
    assert!(
        r.aborted > 5,
        "expected the heap consistency checks to fire, got {} aborts",
        r.aborted
    );
    eprintln!(
        "abort: {} inputs compared byte for byte, {} ASLR dependent in C, {} aborted",
        r.checked, r.aslr_dependent, r.aborted
    );
}

/// Record *why* the inputs above cannot all be compared on stdout: the C program
/// disagrees with itself.  `print_graph` walks the dangling pointer left behind
/// by `delete_node` and prints the tcache `next` field that now sits where the
/// city name was, and that field is `chunk_address >> 12`.
#[test]
fn freed_name_reads_are_not_reproducible_in_c() {
    let input = b"1\nA\n7\nA\n3\n8\n";
    let mut seen = std::collections::HashSet::new();
    let mut runs: Vec<Outcome> = Vec::new();
    for _ in 0..24 {
        let o = run_c(input);
        seen.insert(o.stdout.clone());
        runs.push(o);
    }
    assert!(
        seen.len() > 1,
        "expected the C program's stdout to vary with the heap address, but all \
         24 runs agreed; if ASLR is disabled here this input becomes comparable"
    );

    // Everything else about this input is stable and the Rust program matches
    // it exactly, including the reference count read back out of the freed
    // chunk.  The only divergence is the handful of allocator bytes sitting
    // where the city name used to be: the Rust model leaves that name empty.
    let rust = run_rust(input);
    let marker = b"Choice: Graph with 1 nodes:\nCity: ";
    let cut = find(&rust.stdout, marker).expect("graph header in the Rust output") + marker.len();
    for o in &runs {
        assert_eq!(o.stderr, rust.stderr);
        assert_eq!(o.status, rust.status);
        // Identical up to the name field ...
        assert_eq!(&o.stdout[..cut], &rust.stdout[..cut]);
        // ... then C prints 1 to 8 bytes of `chunk_address >> 12` ...
        let extra = o.stdout.len() - rust.stdout.len();
        assert!(
            (1..=8).contains(&extra),
            "unexpected amount of allocator garbage: {extra} bytes"
        );
        assert!(!o.stdout[cut..cut + extra].contains(&0));
        // ... and the two agree again from there on.
        assert_eq!(&o.stdout[cut + extra..], &rust.stdout[cut..]);
    }
    assert!(String::from_utf8_lossy(&rust.stdout).contains("(ref_count: 0)"));
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
