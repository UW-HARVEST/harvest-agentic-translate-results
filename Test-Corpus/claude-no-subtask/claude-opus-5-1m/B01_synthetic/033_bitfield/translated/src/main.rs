use std::io::{self, Read, Write};

struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new() -> Self {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf).ok();
        Scanner { data: buf, pos: 0 }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.data.len() {
            let c = self.data[self.pos];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Mimic scanf("%u", ...). Returns parsed u32 (wrapping like C unsigned).
    /// On failure, returns None.
    fn scan_u32(&mut self) -> Option<u32> {
        self.skip_ws();
        if self.pos >= self.data.len() {
            return None;
        }
        let mut neg = false;
        let c = self.data[self.pos];
        if c == b'+' {
            self.pos += 1;
        } else if c == b'-' {
            neg = true;
            self.pos += 1;
        }
        let start = self.pos;
        let mut val: u32 = 0;
        while self.pos < self.data.len() {
            let d = self.data[self.pos];
            if d.is_ascii_digit() {
                val = val.wrapping_mul(10).wrapping_add((d - b'0') as u32);
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return None;
        }
        if neg {
            val = 0u32.wrapping_sub(val);
        }
        Some(val)
    }

    /// Mimic scanf("%d", ...). Returns parsed i32 (wrapping like C signed).
    fn scan_i32(&mut self) -> Option<i32> {
        self.skip_ws();
        if self.pos >= self.data.len() {
            return None;
        }
        let mut neg = false;
        let c = self.data[self.pos];
        if c == b'+' {
            self.pos += 1;
        } else if c == b'-' {
            neg = true;
            self.pos += 1;
        }
        let start = self.pos;
        let mut val: u32 = 0;
        while self.pos < self.data.len() {
            let d = self.data[self.pos];
            if d.is_ascii_digit() {
                val = val.wrapping_mul(10).wrapping_add((d - b'0') as u32);
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return None;
        }
        let result = if neg {
            (0u32.wrapping_sub(val)) as i32
        } else {
            val as i32
        };
        Some(result)
    }
}

struct Foo {
    x: u32, // 2-bit
    y: u32, // 3-bit
    b: bool, // 1-bit
    z: i32,
}

fn print_foo(foo: &Foo) {
    // C: printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);
    // bool field promotes to int: 0 or 1.
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let b_val: i32 = if foo.b { 1 } else { 0 };
    write!(out, "{} {} {} {}\n", foo.x, foo.y, b_val, foo.z).unwrap();
}

fn driver(x: u32, y: u32, b: bool, z: i32) {
    let foo = Foo {
        x: x & 0x3,        // 2-bit field
        y: y & 0x7,        // 3-bit field
        b,                 // 1-bit bool: 0 or 1 already
        z,
    };
    print_foo(&foo);
}

fn main() {
    let mut sc = Scanner::new();
    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let mut b: i32 = 0;
    let mut z: i32 = 0;
    if let Some(v) = sc.scan_u32() {
        x = v;
    }
    if let Some(v) = sc.scan_u32() {
        y = v;
    }
    if let Some(v) = sc.scan_i32() {
        b = v;
    }
    if let Some(v) = sc.scan_i32() {
        z = v;
    }
    // !!b in C: nonzero -> true, 0 -> false
    driver(x, y, b != 0, z);
}
