use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

const SYM_INIT_SIZE: usize = 1024;
#[allow(dead_code)]
const TOKEN_SIZE: usize = 1024;
#[allow(dead_code)]
type Token = String;

const FNV_PRIME_32: u32 = 16777619;
const FNV_OFFSET_32: u32 = 2166136261;

pub fn fnv32(token: &str) -> usize {
    let mut hsh: u32 = FNV_OFFSET_32;
    for byte in token.as_bytes() {
        hsh ^= *byte as u32;
        hsh = hsh.wrapping_mul(FNV_PRIME_32);
    }
    hsh as usize
}

pub struct SymTable {
    pub n_items: usize,
    pub n_max: usize,
    pub sym: Vec<Option<String>>,
    pub rev: HashMap<String, usize>,
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
        let mut line = String::new();
        let mut line_no = 1;
        loop {
            line.clear();
            let n = fin.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            let trimmed = line.trim_end_matches(|c| c == '\n' || c == '\r');
            // try to parse as "token\tid"
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(id) = parts[1].parse::<usize>() {
                    self.add(id as i32, parts[0]);
                } else {
                    eprintln!("Invalid input line {}: {}", line_no, trimmed);
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid input"));
                }
            } else if !trimmed.is_empty() {
                eprintln!("Invalid input line {}: {}", line_no, trimmed);
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid input"));
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
        for i in 0..self.n_max {
            if let Some(ref s) = self.sym[i] {
                println!("{}\t{}", s, i);
            }
        }
    }

    pub fn add(&mut self, id: i32, token: &str) -> Option<String> {
        let id_us = id as usize;
        while self.n_max <= id_us {
            self.n_max *= 2;
        }
        if self.sym.len() < self.n_max {
            self.sym.resize(self.n_max, None);
        }
        let token_string = token.to_string();
        self.sym[id_us] = Some(token_string.clone());
        // Also update reverse map
        self.rev.insert(token_string.clone(), id_us);
        self.n_items += 1;
        Some(token_string)
    }

    pub fn getr(&self, token: &str) -> Option<i32> {
        // First check the rev table
        if let Some(&id) = self.rev.get(token) {
            return Some(id as i32);
        }
        // Else linear search through sym
        for i in 0..self.sym.len() {
            if let Some(ref s) = self.sym[i] {
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
        let idx = id as usize;
        if idx >= self.sym.len() {
            return None;
        }
        self.sym[idx].as_deref()
    }

    pub fn compile(&mut self, token: &str) {
        let mut line_no = 1;
        for line in token.split('\n') {
            let trimmed = line.trim_end_matches(|c| c == '\n' || c == '\r');
            if trimmed.is_empty() {
                line_no += 1;
                continue;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(id) = parts[1].parse::<usize>() {
                    self.add(id as i32, parts[0]);
                } else {
                    eprintln!("Invalid input line {}: {}", line_no, trimmed);
                    return;
                }
            }
            line_no += 1;
        }
    }
}
