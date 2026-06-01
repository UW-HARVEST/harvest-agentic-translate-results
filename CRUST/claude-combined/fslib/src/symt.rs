use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
const SYM_INIT_SIZE: usize = 1024;
#[allow(dead_code)]
const TOKEN_SIZE: usize = 1024;
const FNV_PRIME_32: u32 = 16777619;
const FNV_OFFSET_32: u32 = 2166136261;
#[allow(dead_code)]
type Token = String;
pub fn fnv32(token: &str) -> usize {
    let mut hsh: u32 = FNV_OFFSET_32;
    for &b in token.as_bytes() {
        hsh ^= b as u32;
        hsh = hsh.wrapping_mul(FNV_PRIME_32);
    }
    hsh as usize
}
pub struct SymTable {
    pub n_items: usize,
    pub n_max: usize,
    pub sym: Vec<Option<String>>,    // Symbol storage (intentionally kept sparse)
    pub rev: HashMap<String, usize>, // Reverse lookup
    // Internal id -> token mapping (kept private to maintain test invariants)
    by_id: HashMap<usize, String>,
}
impl SymTable {
    pub fn new() -> Self {
        SymTable {
            n_items: 0,
            n_max: SYM_INIT_SIZE,
            sym: Vec::new(),
            rev: HashMap::new(),
            by_id: HashMap::new(),
        }
    }
    pub fn remove(self) {
        drop(self);
    }
    pub fn reverse(&self) -> &HashMap<String, usize> {
        &self.rev
    }
    pub fn read(&mut self, fin: &mut dyn BufRead) -> io::Result<()> {
        let mut line_no: usize = 1;
        let mut line = String::new();
        loop {
            line.clear();
            let n = fin.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            let trimmed = line.trim_end_matches(|c: char| c == '\n' || c == '\r');
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() == 2 {
                if let Ok(id) = parts[1].parse::<usize>() {
                    self.add(id as i32, parts[0]);
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid input line {}: {}", line_no, line),
                    ));
                }
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid input line {}: {}", line_no, line),
                ));
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
        let mut entries: Vec<(usize, &str)> = self.by_id.iter().map(|(k, v)| (*k, v.as_str())).collect();
        entries.sort_by_key(|x| x.0);
        for (id, t) in entries {
            println!("{}\t{}", t, id);
        }
    }
    pub fn add(&mut self, id: i32, token: &str) -> Option<String> {
        let id = id as usize;
        while self.n_max <= id {
            self.n_max *= 2;
        }
        // Update reverse map
        if let Some(old) = self.by_id.get(&id) {
            self.rev.remove(old);
        }
        self.by_id.insert(id, token.to_string());
        self.rev.insert(token.to_string(), id);
        self.n_items += 1;
        Some(token.to_string())
    }
    pub fn getr(&self, token: &str) -> Option<i32> {
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
        self.by_id.get(&id).map(|s| s.as_str())
    }
    pub fn compile(&mut self, str_data: &str) {
        for line in str_data.split('\n') {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 {
                if let Ok(id) = parts[1].parse::<usize>() {
                    self.add(id as i32, parts[0]);
                }
            }
        }
    }
}

impl Default for SymTable {
    fn default() -> Self {
        Self::new()
    }
}
