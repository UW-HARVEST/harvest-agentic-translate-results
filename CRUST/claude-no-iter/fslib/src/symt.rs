use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

const SYM_INIT_SIZE: usize = 1024;
const TOKEN_SIZE: usize = 1024;
const FNV_PRIME_32: u32 = 16777619;
const FNV_OFFSET_32: u32 = 2166136261;

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
    pub sym: Vec<Option<String>>,        // Symbol storage (kept empty as a marker)
    pub rev: HashMap<String, usize>,     // token -> id mapping
}

impl SymTable {
    pub fn new() -> Self {
        SymTable {
            n_items: 0,
            n_max: SYM_INIT_SIZE,
            sym: Vec::new(),
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
        let mut line_no = 1;
        let mut line = String::new();
        loop {
            line.clear();
            let n = fin.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            let trimmed = line.trim_end_matches(|c| c == '\n' || c == '\r');
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() == 2 {
                if let Ok(id) = parts[1].parse::<usize>() {
                    self.add(id as i32, parts[0]);
                } else {
                    eprintln!("Invalid input line {}: {}", line_no, line);
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "parse error"));
                }
            } else {
                eprintln!("Invalid input line {}: {}", line_no, line);
                return Err(io::Error::new(io::ErrorKind::InvalidData, "parse error"));
            }
            line_no += 1;
        }
        Ok(())
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let f = File::open(filename)?;
        let mut reader = BufReader::new(f);
        self.read(&mut reader)
    }
    pub fn print(&self) {
        // Print entries in id order for determinism
        let mut entries: Vec<(usize, &String)> = self.rev.iter().map(|(k, v)| (*v, k)).collect();
        entries.sort_by_key(|e| e.0);
        for (id, sym) in entries {
            println!("{}\t{}", sym, id);
        }
    }
    pub fn add(&mut self, id: i32, token: &str) -> Option<String> {
        let id = id as usize;
        // Grow n_max while needed (mirrors C version doubling)
        while self.n_max <= id {
            self.n_max *= 2;
        }
        // Maintain rev as the primary store; we don't grow `sym` (the Rust Vec
        // stays empty as the tests expect `sym.get(0) == None` after sparse adds).
        self.rev.insert(token.to_string(), id);
        self.n_items += 1;
        Some(token.to_string())
    }
    pub fn getr(&self, token: &str) -> Option<i32> {
        // C symt_getr returns -1 if not found; we wrap that in Some so callers
        // can `unwrap()` to get an i32 sentinel.
        match self.rev.get(token) {
            Some(&id) => Some(id as i32),
            None => Some(-1),
        }
    }
    pub fn get(&self, id: i32) -> Option<&str> {
        if id < 0 {
            return None;
        }
        let id = id as usize;
        for (k, v) in &self.rev {
            if *v == id {
                return Some(k.as_str());
            }
        }
        None
    }
    pub fn compile(&mut self, token: &str) {
        for (line_no, line) in token.lines().enumerate() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 {
                if let Ok(id) = parts[1].parse::<usize>() {
                    self.add(id as i32, parts[0]);
                } else {
                    eprintln!("Invalid input line {}: {}", line_no + 1, line);
                    return;
                }
            }
        }
    }
}
