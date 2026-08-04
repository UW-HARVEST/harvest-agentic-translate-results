use std::collections::HashMap;
use std::io::{self, BufRead, BufReader};
use std::fs::File;
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
    pub sym: Vec<Option<String>>,    // Symbol storage
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
    pub fn read(&mut self, fin: &mut dyn BufRead) -> io::Result<()> {
        let mut line_no = 1;
        let mut line = String::new();
        loop {
            line.clear();
            let n = fin.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
            // C uses sscanf("%s\t%zu") - %s is whitespace-delimited
            let mut parts = trimmed.split_whitespace();
            let token = parts.next();
            let id_str = parts.next();
            match (token, id_str) {
                (Some(tok), Some(id_s)) => {
                    if let Ok(id) = id_s.parse::<usize>() {
                        self.add(id as i32, tok);
                    } else {
                        eprintln!("Invalid input line {}: {}", line_no, line);
                        std::process::exit(1);
                    }
                }
                _ => {
                    eprintln!("Invalid input line {}: {}", line_no, line);
                    std::process::exit(1);
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
        for i in 0..self.n_max {
            if let Some(ref s) = self.sym[i] {
                println!("{}\t{}", s, i);
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
            for _ in s..self.n_max {
                self.sym.push(None);
            }
        }
        let t = token.to_string();
        self.sym[id] = Some(t.clone());
        self.n_items += 1;
        Some(t)
    }
    pub fn getr(&self, token: &str) -> Option<i32> {
        // First try the rev table
        if !self.rev.is_empty() {
            if let Some(&id) = self.rev.get(token) {
                return Some(id as i32);
            }
            return None;
        }
        // fallback - linear search
        for i in 0..self.n_max {
            if let Some(ref s) = self.sym[i] {
                if s == token {
                    return Some(i as i32);
                }
            }
        }
        None
    }
    pub fn get(&self, id: i32) -> Option<&str> {
        let id = id as usize;
        if id > self.n_items {
            // Note: C uses ">" so id == n_items is valid; this matches the behavior
        }
        if id >= self.sym.len() {
            return None;
        }
        if id > self.n_items {
            return None;
        }
        self.sym[id].as_deref()
    }
    pub fn compile(&mut self, token: &str) {
        let mut line_no = 1;
        for tok in token.split('\n') {
            if tok.is_empty() {
                line_no += 1;
                continue;
            }
            let mut parts = tok.split_whitespace();
            let t = parts.next();
            let id_str = parts.next();
            match (t, id_str) {
                (Some(t), Some(id_s)) => {
                    if let Ok(id) = id_s.parse::<usize>() {
                        self.add(id as i32, t);
                    } else {
                        eprintln!("Invalid input line {}: {}", line_no, tok);
                        std::process::exit(1);
                    }
                }
                _ => {
                    eprintln!("Invalid input line {}: {}", line_no, tok);
                    std::process::exit(1);
                }
            }
            line_no += 1;
        }
    }
}
// build reverse table
impl SymTable {
    pub fn build_reverse(&mut self) {
        self.rev.clear();
        for i in 0..self.n_max {
            if let Some(ref s) = self.sym[i] {
                let truncated: String = s.chars().take(TOKEN_SIZE - 1).collect();
                self.rev.insert(truncated, i);
            }
        }
    }
}
