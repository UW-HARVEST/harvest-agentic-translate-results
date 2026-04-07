use std::io::Read;

struct RoutingDirective {
    time_stamp: u32,
    luggage_id: String,
    flight_id: String,
    departure: String,
    arrival: String,
    comments: String,
}

fn matches(expected: &str, actual: &str) -> bool {
    expected.starts_with('-') || expected == actual
}

fn supersedes(directives: &[RoutingDirective], start: usize, luggage_id: &str, departure: &str) -> bool {
    for d in &directives[start..] {
        if d.luggage_id == luggage_id {
            return d.departure == departure;
        }
    }
    false
}

fn superseded(directives: &[RoutingDirective], idx: usize) -> bool {
    supersedes(directives, idx + 1, &directives[idx].luggage_id, &directives[idx].departure)
}

struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new() -> Self {
        let mut data = Vec::new();
        std::io::stdin().read_to_end(&mut data).unwrap();
        Scanner { data, pos: 0 }
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) -> bool {
        // Like scanf " " — skip zero or more whitespace. Returns false if we hit EOF without skipping anything and there's nothing left.
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                self.pos += 1;
            } else {
                return true;
            }
        }
        // Reached EOF
        false
    }

    // scanf("%d ", ...) — read signed int (stored as unsigned), then skip whitespace
    fn scan_int_ws(&mut self) -> Option<u32> {
        if self.at_eof() { return None; }
        // skip leading whitespace (scanf %d skips leading whitespace)
        self.skip_whitespace();
        if self.at_eof() { return None; }

        let mut neg = false;
        if self.peek() == Some(b'-') {
            neg = true;
            self.pos += 1;
        } else if self.peek() == Some(b'+') {
            self.pos += 1;
        }

        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start { return None; }

        let s: String = self.data[start..self.pos].iter().map(|&b| b as char).collect();
        let val: i32 = s.parse().ok()?;
        let val = if neg { -val } else { val };
        // Trailing space in format: skip whitespace
        self.skip_whitespace();
        Some(val as u32)
    }

    // Read chars matching a predicate, up to max_len
    fn scan_charset(&mut self, max_len: usize, pred: fn(u8) -> bool) -> Option<String> {
        if self.at_eof() { return None; }
        let start = self.pos;
        let mut count = 0;
        while count < max_len {
            if let Some(b) = self.peek() {
                if pred(b) {
                    self.pos += 1;
                    count += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if self.pos == start {
            // scanf scanset returns EOF if no chars matched and we're at EOF, or 0 matches
            // Actually in C, if no chars match a scanset, scanf returns the count so far (not EOF).
            // But the C code checks == EOF. If no chars match but we're not at EOF, scanf doesn't return EOF.
            // If we ARE at EOF and no chars match, scanf returns EOF.
            if self.at_eof() { return None; }
            // No match but not EOF — return empty string (scanf would return count so far, not EOF)
            return Some(String::new());
        }
        Some(self.data[start..self.pos].iter().map(|&b| b as char).collect())
    }

    // scanf("%80[^\n]", comments) — read up to 80 non-newline chars
    fn scan_comments(&mut self) -> Option<String> {
        self.scan_charset(80, |b| b != b'\n')
    }
}

fn is_alnum_upper(b: u8) -> bool {
    b.is_ascii_uppercase() || b.is_ascii_digit()
}

fn is_alpha_upper(b: u8) -> bool {
    b.is_ascii_uppercase()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        eprintln!("Command line error: 4 arguments expected");
        std::process::exit(1);
    }

    let mut directives: Vec<RoutingDirective> = Vec::new();
    let mut scanner = Scanner::new();

    loop {
        let time_stamp = match scanner.scan_int_ws() {
            Some(v) => v,
            None => break,
        };
        let luggage_id = match scanner.scan_charset(8, is_alnum_upper) {
            Some(s) => s,
            None => break,
        };
        // skip whitespace between luggage_id and flight_id
        scanner.skip_whitespace();
        let flight_id = match scanner.scan_charset(6, is_alnum_upper) {
            Some(s) => s,
            None => break,
        };
        // trailing space after flight_id
        scanner.skip_whitespace();
        let departure = match scanner.scan_charset(3, is_alpha_upper) {
            Some(s) => s,
            None => break,
        };
        // space between departure and arrival
        scanner.skip_whitespace();
        let arrival = match scanner.scan_charset(3, is_alpha_upper) {
            Some(s) => s,
            None => break,
        };
        let comments = match scanner.scan_comments() {
            Some(s) => s,
            None => break,
        };

        let new_dir = RoutingDirective {
            time_stamp,
            luggage_id,
            flight_id,
            departure,
            arrival,
            comments,
        };

        // Insert sorted by time_stamp (stable: insert after equal elements)
        let insert_pos = directives.iter().position(|d| d.time_stamp > new_dir.time_stamp);
        match insert_pos {
            Some(pos) => directives.insert(pos, new_dir),
            None => directives.push(new_dir),
        }
    }

    for i in 0..directives.len() {
        let d = &directives[i];
        if !superseded(&directives, i)
            && matches(&args[1], &d.luggage_id)
            && matches(&args[2], &d.flight_id)
            && matches(&args[3], &d.departure)
            && matches(&args[4], &d.arrival)
        {
            println!(
                "{:010} {} {} {} {} {}",
                d.time_stamp, d.luggage_id, d.flight_id, d.departure, d.arrival, d.comments
            );
        }
    }

    std::process::exit(0);
}
