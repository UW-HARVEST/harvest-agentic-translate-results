#!/bin/bash
# Compare the REAL Rust strtoul_base10 + validation decision from
# translation/src/main.rs against glibc's strtoul, over a large battery of
# byte-exact inputs.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/scratch"

python3 gen_str_cases.py > str_cases.hex
gcc -O0 -o oracle_str_c oracle_str.c

sed 's/^fn main() {$/fn unused_main() {/' ../translation/src/main.rs > oracle_str_gen.rs
cat >> oracle_str_gen.rs <<'EOF'

// ---- oracle harness (appended) -------------------------------------------
use std::io::BufRead;

fn unhex(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut out = String::new();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let h = line.trim_end().as_bytes();
        let mut buf = Vec::with_capacity(h.len() / 2);
        let mut i = 0;
        while i + 1 < h.len() {
            buf.push((unhex(h[i]) << 4) | unhex(h[i + 1]));
            i += 2;
        }

        let parsed = strtoul_base10(&buf);
        let temp_seed = parsed.value;
        out.push_str(&format!(
            "val={} off={} erange={}",
            temp_seed,
            parsed.end,
            if parsed.erange { 1 } else { 0 }
        ));
        if parsed.end != buf.len() || parsed.erange || temp_seed > UINT_MAX {
            out.push_str(" decision=err\n");
        } else {
            out.push_str(&format!(" decision=ok seed={}\n", temp_seed as u32));
        }
    }
    print!("{}", out);
}
EOF

rustc -O --edition 2021 -A dead_code -A unused_imports -o oracle_str_rs oracle_str_gen.rs

./oracle_str_c  < str_cases.hex > str_c.txt
./oracle_str_rs < str_cases.hex > str_rs.txt

echo "cases: $(wc -l < str_cases.hex)"
if diff -u str_c.txt str_rs.txt > str_diff.txt; then
  echo "STRTOUL ORACLE MATCH ($(wc -l < str_c.txt) lines)"
else
  echo "STRTOUL ORACLE MISMATCH:"
  paste -d'|' <(cut -c1-0 /dev/null) /dev/null 2>/dev/null || true
  # show mismatching case index + the raw input
  nl -ba str_c.txt > /tmp/c.nl; nl -ba str_rs.txt > /tmp/r.nl
  diff /tmp/c.nl /tmp/r.nl | head -60
  echo "--- offending inputs ---"
  diff <(cat str_c.txt) <(cat str_rs.txt) | grep -oP '^\d+' | sort -un | while read -r ln; do
     printf 'line %s input=' "$ln"; sed -n "${ln}p" str_cases.hex | head -c 120; echo
  done
  exit 1
fi
