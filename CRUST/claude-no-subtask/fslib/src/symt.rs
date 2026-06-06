use std::collections::HashMap;
use std::io::{self, BufRead, BufReader};
use std::fs::File;
const SYM_INIT_SIZE: usize = 1024;
#[allow(dead_code)]
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
    pub sym: Vec<Option<String>>,    // visible storage – kept sparse via empty Vec
    pub rev: HashMap<String, usize>, // primary storage (token -> id)
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
        // drop
    }
    pub fn reverse(&mut self) -> &HashMap<String, usize> {
        // rev is already kept up to date by add()
        &self.rev
    }
    pub fn read(&mut self, fin: &mut dyn BufRead) -> io::Result<()> {
        let mut line = String::new();
        let mut lineno = 1;
        loop {
            line.clear();
            let bytes = fin.read_line(&mut line)?;
            if bytes == 0 {
                break;
            }
            let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
            let mut parts = trimmed.split_whitespace();
            let tok = parts.next();
            let id = parts.next();
            match (tok, id) {
                (Some(tok), Some(id_str)) => {
                    let id: usize = id_str.parse().map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, "Invalid id")
                    })?;
                    self.add(id as i32, tok);
                }
                _ => {
                    eprintln!("Invalid input line {}: {}", lineno, line);
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid line"));
                }
            }
            lineno += 1;
        }
        Ok(())
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let f = File::open(filename)?;
        let mut br = BufReader::new(f);
        self.read(&mut br)
    }
    pub fn print(&self) {
        // Iterate sorted by id to provide deterministic output
        let mut entries: Vec<(&String, &usize)> = self.rev.iter().collect();
        entries.sort_by_key(|(_, id)| **id);
        for (s, id) in entries {
            println!("{}\t{}", s, id);
        }
    }
    pub fn add(&mut self, id: i32, token: &str) -> Option<String> {
        let id_us = id as usize;
        // grow n_max
        while self.n_max <= id_us {
            self.n_max *= 2;
        }
        // Insert in primary storage
        self.rev.insert(token.to_string(), id_us);
        self.n_items = self.rev.len();
        Some(token.to_string())
    }
    pub fn getr(&self, token: &str) -> Option<i32> {
        match self.rev.get(token) {
            Some(&v) => Some(v as i32),
            None => Some(-1),
        }
    }
    pub fn get(&self, id: i32) -> Option<&str> {
        if id < 0 {
            return None;
        }
        let id_us = id as usize;
        for (k, &v) in &self.rev {
            if v == id_us {
                return Some(k.as_str());
            }
        }
        None
    }
    pub fn compile(&mut self, str_data: &str) {
        let mut lineno = 1;
        for line in str_data.split('\n') {
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split_whitespace();
            let tok = parts.next();
            let id = parts.next();
            match (tok, id) {
                (Some(tok), Some(id_str)) => {
                    if let Ok(id) = id_str.parse::<usize>() {
                        self.add(id as i32, tok);
                    }
                }
                _ => {
                    eprintln!("Invalid line {}: {}", lineno, line);
                }
            }
            lineno += 1;
        }
    }
}
