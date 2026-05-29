use std::collections::HashMap;
use std::io::{self, BufRead};
const SYM_INIT_SIZE: usize = 1024;
#[allow(dead_code)]
const TOKEN_SIZE: usize = 1024;
const FNV_PRIME_32: u32 = 16777619;
const FNV_OFFSET_32: u32 = 2166136261;
pub fn fnv32(token: &str) -> usize {
    let mut hsh: u32 = FNV_OFFSET_32;
    for byte in token.bytes() {
        hsh ^= byte as u32;
        hsh = hsh.wrapping_mul(FNV_PRIME_32);
    }
    hsh as usize
}
pub struct SymTable {
    pub n_items: usize,
    pub n_max: usize,
    pub sym: Vec<Option<String>>, // Symbol storage (kept sparse / empty)
    pub rev: HashMap<String, usize>, // Reverse lookup: token -> id (also primary store)
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
        // Drop in Rust handles cleanup
    }
    pub fn reverse(&mut self) -> &HashMap<String, usize> {
        // rev is maintained on every add already; just return it
        &self.rev
    }
    pub fn read(&mut self, fin: &mut dyn BufRead) -> io::Result<()> {
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
                }
            }
        }
        Ok(())
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let file = std::fs::File::open(filename)?;
        let mut reader = std::io::BufReader::new(file);
        self.read(&mut reader)
    }
    pub fn print(&self) {
        let mut entries: Vec<(usize, &String)> = self.rev.iter().map(|(k, v)| (*v, k)).collect();
        entries.sort_by_key(|(id, _)| *id);
        for (id, token) in entries {
            println!("{}\t{}", token, id);
        }
    }
    pub fn add(&mut self, id: i32, token: &str) -> Option<String> {
        let id_us = id as usize;
        while self.n_max <= id_us {
            self.n_max *= 2;
        }
        self.rev.insert(token.to_string(), id_us);
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
        let id_us = id as usize;
        for (k, v) in &self.rev {
            if *v == id_us {
                return Some(k.as_str());
            }
        }
        None
    }
    pub fn compile(&mut self, token: &str) {
        for line in token.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 {
                if let Ok(id) = parts[1].parse::<usize>() {
                    self.add(id as i32, parts[0]);
                }
            }
        }
    }
}
