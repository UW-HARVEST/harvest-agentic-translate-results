use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

const SYM_INIT_SIZE: usize = 1024;
const TOKEN_SIZE: usize = 1024;
type Token = String;

pub fn fnv32(token: &str) -> usize {
    let mut hash: u32 = 2166136261;
    for byte in token.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash as usize
}

pub struct SymTable {
    pub n_items: usize,
    pub n_max: usize,
    pub sym: Vec<Option<String>>,
    pub rev: HashMap<String, usize>,
}

impl SymTable {
    pub fn new() -> Self {
        Self {
            n_items: 0,
            n_max: SYM_INIT_SIZE,
            sym: vec![None; SYM_INIT_SIZE],
            rev: HashMap::new(),
        }
    }

    pub fn remove(self) {}

    pub fn reverse(&self) -> &HashMap<String, usize> {
        &self.rev
    }

    pub fn read(&mut self, fin: &mut dyn BufRead) -> io::Result<()> {
        for (idx, line) in fin.lines().enumerate() {
            let line = line?;
            let mut parts = line.splitn(2, '\t');
            let token = parts.next().unwrap_or_default();
            let id = parts
                .next()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid input line {}", idx + 1)))?
                .trim()
                .parse::<i32>()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid input line {}", idx + 1)))?;
            self.add(id, token);
        }
        Ok(())
    }

    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let file = File::open(filename)?;
        let mut reader = BufReader::new(file);
        self.read(&mut reader)
    }

    pub fn print(&self) {
        for (idx, token) in self.sym.iter().enumerate() {
            if let Some(token) = token {
                println!("{}\t{}", token, idx);
            }
        }
    }

    pub fn add(&mut self, id: i32, token: &str) -> Option<String> {
        if id < 0 {
            return None;
        }

        let id = id as usize;
        let old_n_max = self.n_max;
        while self.n_max <= id {
            self.n_max *= 2;
        }
        if self.n_max > old_n_max {
            self.sym.resize(self.n_max, None);
        }

        let token = token[..token.len().min(TOKEN_SIZE.saturating_sub(1))].to_string();
        self.sym[id] = Some(token.clone());
        self.rev.insert(token.clone(), id);
        self.n_items += 1;
        Some(token)
    }

    pub fn getr(&self, token: &str) -> Option<i32> {
        self.rev.get(token).copied().map(|id| id as i32)
    }

    pub fn get(&self, id: i32) -> Option<&str> {
        if id < 0 || id as usize > self.n_items {
            return None;
        }
        self.sym.get(id as usize)?.as_deref()
    }

    pub fn compile(&mut self, token: &str) {
        for (idx, line) in token.lines().enumerate() {
            let mut parts = line.splitn(2, '\t');
            let sym = parts.next().unwrap_or_default();
            if let Some(id) = parts.next().and_then(|value| value.trim().parse::<i32>().ok()) {
                self.add(id, sym);
            } else {
                eprintln!("Invalid input line {}: {}", idx + 1, line);
                std::process::exit(1);
            }
        }
    }
}
