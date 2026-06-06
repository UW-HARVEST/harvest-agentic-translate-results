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
    for b in token.as_bytes() {
        hsh ^= *b as u32;
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
        let mut sym = Vec::with_capacity(SYM_INIT_SIZE);
        for _ in 0..SYM_INIT_SIZE {
            sym.push(None);
        }
        SymTable {
            n_items: 0,
            n_max: SYM_INIT_SIZE,
            sym,
            rev: HashMap::new(),
        }
    }
    pub fn remove(self) {
        drop(self);
    }
    pub fn reverse(&self) -> &HashMap<String, usize> {
        &self.rev
    }
    pub fn rebuild_reverse(&mut self) {
        self.rev.clear();
        for (i, sym) in self.sym.iter().enumerate() {
            if let Some(s) = sym {
                self.rev.insert(s.clone(), i);
            }
        }
    }
    pub fn read(&mut self, fin: &mut dyn BufRead) -> io::Result<()> {
        let mut line = String::new();
        let mut line_no = 1;
        loop {
            line.clear();
            let n = fin.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
            // Parse "%s\t%zu" - C uses scanf so it splits by whitespace, but the output uses "\t"
            // We split by whitespace: token then number
            let mut parts = trimmed.split_whitespace();
            if let (Some(token), Some(id_str)) = (parts.next(), parts.next()) {
                if let Ok(id) = id_str.parse::<i32>() {
                    self.add(id, token);
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
        for (i, sym) in self.sym.iter().enumerate() {
            if let Some(s) = sym {
                println!("{}\t{}", s, i);
            }
        }
    }
    pub fn add(&mut self, id: i32, token: &str) -> Option<String> {
        if id < 0 {
            return None;
        }
        let id = id as usize;
        let mut s = self.n_max;
        while self.n_max <= id {
            self.n_max *= 2;
        }
        if self.n_max > s {
            // grow
            self.sym.resize(self.n_max, None);
            // s was already valid
            let _ = s;
        }
        let t = token.to_string();
        self.sym[id] = Some(t.clone());
        self.n_items += 1;
        // also update rev if previously built
        if !self.rev.is_empty() {
            self.rev.insert(t.clone(), id);
        }
        Some(t)
    }
    pub fn getr(&self, token: &str) -> Option<i32> {
        // Search symbol table for the token
        // First check reverse map
        if let Some(&id) = self.rev.get(token) {
            return Some(id as i32);
        }
        // Linear search if reverse not built
        for (i, sym) in self.sym.iter().enumerate() {
            if let Some(s) = sym {
                if s == token {
                    return Some(i as i32);
                }
            }
        }
        None
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
        // Compile from a multi-line string
        for line in token.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut parts = trimmed.split_whitespace();
            if let (Some(tok), Some(id_str)) = (parts.next(), parts.next()) {
                if let Ok(id) = id_str.parse::<i32>() {
                    self.add(id, tok);
                }
            }
        }
    }
}
