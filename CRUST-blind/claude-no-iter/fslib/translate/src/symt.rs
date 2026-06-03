use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
const SYM_INIT_SIZE: usize = 1024;
const TOKEN_SIZE: usize = 1024;
const FNV_PRIME_32: u32 = 16777619;
const FNV_OFFSET_32: u32 = 2166136261;
type Token = String;
pub fn fnv32(token: &str) -> usize {
    let mut hsh: u32 = FNV_OFFSET_32;
    for b in token.bytes() {
        hsh ^= b as u32;
        hsh = hsh.wrapping_mul(FNV_PRIME_32);
    }
    hsh as usize
}
pub struct SymTable {
    pub n_items: usize,
    pub n_max: usize,
    pub sym: Vec<Option<String>>, // Symbol storage
    pub rev: HashMap<String, usize>, // Reverse lookup
}
impl SymTable {
    pub fn new() -> Self {
        SymTable {
            n_items: 0,
            n_max: SYM_INIT_SIZE,
            sym: (0..SYM_INIT_SIZE).map(|_| None).collect(),
            rev: HashMap::new(),
        }
    }
    pub fn remove(self) {
        drop(self);
    }
    pub fn reverse(&self) -> &HashMap<String, usize> {
        &self.rev
    }
    pub fn read(&mut self, fin: &mut dyn BufRead) -> io::Result<()> {
        let mut line_no = 1usize;
        let mut buf = String::new();
        loop {
            buf.clear();
            let n = fin.read_line(&mut buf)?;
            if n == 0 {
                break;
            }
            let trimmed = buf.trim_end_matches(|c| c == '\n' || c == '\r');
            // Try to parse "token\tid"
            let mut parts = trimmed.split_whitespace();
            let token = parts.next();
            let id_str = parts.next();
            match (token, id_str) {
                (Some(t), Some(s)) => {
                    if let Ok(id) = s.parse::<usize>() {
                        self.add(id as i32, t);
                    } else {
                        eprintln!("Invalid input line {}: {}", line_no, buf);
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "parse error"));
                    }
                }
                _ => {
                    eprintln!("Invalid input line {}: {}", line_no, buf);
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "parse error"));
                }
            }
            line_no += 1;
        }
        Ok(())
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let f = File::open(filename)?;
        let mut br = BufReader::new(f);
        self.read(&mut br)
    }
    pub fn print(&self) {
        for (i, s) in self.sym.iter().enumerate() {
            if let Some(t) = s {
                println!("{}\t{}", t, i);
            }
        }
    }
    pub fn add(&mut self, id: i32, token: &str) -> Option<String> {
        let id = id as usize;
        let s = self.n_max;
        while self.n_max <= id {
            self.n_max *= 2;
        }
        if self.n_max > s {
            self.sym.resize(self.n_max, None);
        }
        let t = token.to_string();
        self.sym[id] = Some(t.clone());
        self.rev.insert(t.clone(), id);
        self.n_items += 1;
        Some(t)
    }
    pub fn getr(&self, token: &str) -> Option<i32> {
        self.rev.get(token).map(|&v| v as i32)
    }
    pub fn get(&self, id: i32) -> Option<&str> {
        if id < 0 {
            return None;
        }
        let id = id as usize;
        if id >= self.sym.len() {
            return None;
        }
        self.sym[id].as_deref()
    }
    pub fn compile(&mut self, token: &str) {
        // Parse a multi-line string of "token\tid" entries
        let mut line_no = 1usize;
        for line in token.split('\n') {
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split_whitespace();
            let tok = parts.next();
            let id_str = parts.next();
            match (tok, id_str) {
                (Some(t), Some(s)) => {
                    if let Ok(id) = s.parse::<usize>() {
                        self.add(id as i32, t);
                    } else {
                        eprintln!("Invalid input line {}: {}", line_no, line);
                    }
                }
                _ => {
                    eprintln!("Invalid input line {}: {}", line_no, line);
                }
            }
            line_no += 1;
        }
    }
}
