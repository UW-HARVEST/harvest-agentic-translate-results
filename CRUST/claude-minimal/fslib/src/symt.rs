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
        hsh = hsh ^ (*b as u32);
        hsh = hsh.wrapping_mul(FNV_PRIME_32);
    }
    hsh as usize
}
pub struct SymTable {
    pub n_items: usize,
    pub n_max: usize,
    pub sym: Vec<Option<String>>,
    pub rev: HashMap<String, usize>,
    fwd: HashMap<i32, String>,
}
impl SymTable {
    pub fn new() -> Self {
        SymTable {
            n_items: 0,
            n_max: SYM_INIT_SIZE,
            sym: Vec::new(),
            rev: HashMap::new(),
            fwd: HashMap::new(),
        }
    }
    pub fn remove(self) {
        // drop on going out of scope
        drop(self);
    }
    pub fn reverse(&mut self) -> &HashMap<String, usize> {
        self.rev.clear();
        for (id, tok) in self.fwd.iter() {
            self.rev.insert(tok.clone(), *id as usize);
        }
        &self.rev
    }
    pub fn read(&mut self, fin: &mut dyn BufRead) -> io::Result<()> {
        let mut line_no = 1;
        let mut buf = String::new();
        loop {
            buf.clear();
            let n = fin.read_line(&mut buf)?;
            if n == 0 { break; }
            let trimmed = buf.trim_end();
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() == 2 {
                let token = parts[0];
                let id: usize = parts[1].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad id"))?;
                self.add(id as i32, token);
            } else {
                eprintln!("Invalid input line {}: {}", line_no, buf);
                std::process::exit(1);
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
        let mut keys: Vec<i32> = self.fwd.keys().cloned().collect();
        keys.sort();
        for k in keys {
            if let Some(v) = self.fwd.get(&k) {
                println!("{}\t{}", v, k);
            }
        }
    }
    pub fn add(&mut self, id: i32, token: &str) -> Option<String> {
        let uid = id as usize;
        // Track n_max doubling like C
        while self.n_max <= uid {
            self.n_max *= 2;
        }
        self.fwd.insert(id, token.to_string());
        self.n_items += 1;
        Some(token.to_string())
    }
    pub fn getr(&self, token: &str) -> Option<i32> {
        match self.rev.get(token) {
            Some(&v) => Some(v as i32),
            None => Some(-1),
        }
    }
    pub fn get(&self, id: i32) -> Option<&str> {
        if id < 0 { return None; }
        match self.fwd.get(&id) {
            Some(s) => Some(s.as_str()),
            None => None,
        }
    }
    pub fn compile(&mut self, token: &str) {
        let mut line = 1;
        for tok in token.split('\n') {
            if tok.trim().is_empty() {
                line += 1;
                continue;
            }
            let parts: Vec<&str> = tok.split_whitespace().collect();
            if parts.len() == 2 {
                let token = parts[0];
                if let Ok(id) = parts[1].parse::<usize>() {
                    self.add(id as i32, token);
                } else {
                    eprintln!("Invalid input line {}: {}", line, tok);
                    std::process::exit(1);
                }
            } else {
                eprintln!("Invalid input line {}: {}", line, tok);
                std::process::exit(1);
            }
            line += 1;
        }
    }
}
