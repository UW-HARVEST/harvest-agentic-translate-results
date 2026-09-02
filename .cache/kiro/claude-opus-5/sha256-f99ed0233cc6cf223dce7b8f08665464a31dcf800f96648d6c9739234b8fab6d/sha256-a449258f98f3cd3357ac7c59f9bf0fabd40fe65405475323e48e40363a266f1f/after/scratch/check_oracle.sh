#!/bin/bash
# Build a Rust oracle that reuses the REAL code from translation/src/main.rs
# (renaming its `main` so we can bolt our own on) and diff it against the C
# oracle.  This checks the rand() stream and the arithmetic kernel in seconds
# instead of the ~8 minutes a full run costs.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/scratch"

gcc -O0 -o oracle_c oracle.c

# Take the real translation source, rename its entry point, append our own.
sed 's/^fn main() {$/fn unused_main() {/' ../translation/src/main.rs > oracle_gen.rs
cat >> oracle_gen.rs <<'EOF'

// ---- oracle harness (appended) -------------------------------------------
fn kernel_once(x: i32) -> i32 {
    let mut buf = [x];
    perform_expensive_operations(&mut buf);
    buf[0]
}

fn main() {
    let seeds: [u32; 20] = [
        0, 1, 2, 3, 5, 42, 12345, 65535, 127773, 2147483646,
        2147483647, 2147483648, 2147483649, 3000000000, 4294967294,
        4294967295, 16807, 999999999, 1000000000, 2000000000,
    ];
    for s in seeds {
        let mut r = GlibcRand::new(s);
        print!("SEED {}:", s);
        for _ in 0..12 {
            print!(" {}", r.next_i32());
        }
        println!();
    }

    let vals: [i32; 28] = [
        0, 1, -1, 2, -2, 3, -3, 7, -7, 6, -6, 8, -8,
        i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1,
        1073741824, -1073741824, 127773, -127773,
        16807, 2147483646, -2147483647, 100, -100, 12345, -12345,
    ];
    for v in vals {
        println!("K {} -> {}", v, kernel_once(v));
    }
}
EOF

rustc -O --edition 2021 -A dead_code -A unused_imports -o oracle_rs oracle_gen.rs 2>&1 | grep -E "^error" || true
rustc -O --edition 2021 -A dead_code -A unused_imports -o oracle_rs oracle_gen.rs

./oracle_c | grep -v '^TRACE' > oracle_c.txt
./oracle_rs > oracle_rs.txt

if diff -u oracle_c.txt oracle_rs.txt > oracle_diff.txt; then
  echo "ORACLE MATCH: $(wc -l < oracle_c.txt) lines identical"
else
  echo "ORACLE MISMATCH:"
  head -40 oracle_diff.txt
  exit 1
fi
