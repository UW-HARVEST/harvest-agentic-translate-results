use std::collections::HashMap;
use std::io::{self, BufRead};

const SYM_INIT_SIZE: usize = 1024;
#[allow(dead_code)]
const TOKEN_SIZE: usize = 1024;

const FNV_PRIME_32: u32 = 16777619;
const FNV_OFFSET_32: u32 = 2166136261;

#[allow(dead_code)]
type Token = String;

pub fn fnv32(token: &str) -> usize {
    let mut hsh = FNV_OFFSET_32;
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
        Self {
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
        let mut line_no: usize = 1;
        let mut line = String::new();
        loop {
            line.clear();
            let n = fin.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            let trimmed = line.trim_end_matches('\n');
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() == 2 {
                if let Ok(id) = parts[1].parse::<usize>() {
                    self.add(id as i32, parts[0]);
                } else {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "parse"));
                }
            } else {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "parse"));
            }
            line_no += 1;
        }
        let _ = line_no;
        Ok(())
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let f = std::fs::File::open(filename)?;
        let mut buf = std::io::BufReader::new(f);
        self.read(&mut buf)
    }
    pub fn print(&self) {
        // Print in insertion order: iterate rev and sort by id
        let mut entries: Vec<(&String, &usize)> = self.rev.iter().collect();
        entries.sort_by_key(|&(_, id)| *id);
        for (token, id) in entries {
            println!("{}\t{}", token, id);
        }
    }
    pub fn add(&mut self, id: i32, token: &str) -> Option<String> {
        let idx = id as usize;
        // Match C behavior: grow n_max
        while self.n_max <= idx {
            self.n_max *= 2;
        }
        // Insert into rev for storage
        self.rev.insert(token.to_string(), idx);
        self.n_items += 1;
        None
    }
    pub fn getr(&self, token: &str) -> Option<i32> {
        match self.rev.get(token) {
            Some(&i) => Some(i as i32),
            None => Some(-1),
        }
    }
    pub fn get(&self, id: i32) -> Option<&str> {
        if id < 0 {
            return None;
        }
        let idx = id as usize;
        // Find a token that maps to this id
        for (token, &i) in self.rev.iter() {
            if i == idx {
                return Some(token.as_str());
            }
        }
        None
    }
    pub fn compile(&mut self, token: &str) {
        let mut line_no: usize = 1;
        for line in token.split('\n') {
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(id) = parts[1].parse::<usize>() {
                    self.add(id as i32, parts[0]);
                } else {
                    eprintln!("Invalid input line {}: {}", line_no, line);
                }
            }
            line_no += 1;
        }
    }
}
