use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
const SYM_INIT_SIZE: usize = 1024;
const TOKEN_SIZE: usize = 1024;
type Token = String;
pub fn fnv32(token: &str) -> usize {
    let mut hsh: u32 = 2166136261;
    for b in token.bytes() {
        hsh ^= b as u32;
        hsh = hsh.wrapping_mul(16777619);
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
        SymTable {
            n_items: 0,
            n_max: SYM_INIT_SIZE,
            sym: vec![None; SYM_INIT_SIZE],
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
        for line_result in fin.lines() {
            let line = line_result?;
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() == 2 {
                if let Ok(id) = parts[1].parse::<usize>() {
                    self.add(id as i32, parts[0]);
                } else {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Invalid input: {}", line)));
                }
            } else {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Invalid input: {}", line)));
            }
        }
        Ok(())
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let file = File::open(filename)?;
        let mut reader = BufReader::new(file);
        self.read(&mut reader)
    }
    pub fn print(&self) {
        for (i, s) in self.sym.iter().enumerate() {
            if let Some(token) = s {
                println!("{}\t{}", token, i);
            }
        }
    }
    pub fn add(&mut self, id: i32, token: &str) -> Option<String> {
        let id = id as usize;
        while self.n_max <= id {
            self.n_max *= 2;
        }
        if self.sym.len() < self.n_max {
            self.sym.resize(self.n_max, None);
        }
        let t = token.to_string();
        self.sym[id] = Some(t.clone());
        self.rev.insert(t.clone(), id);
        self.n_items += 1;
        Some(t)
    }
    pub fn getr(&self, token: &str) -> Option<i32> {
        self.rev.get(token).map(|&id| id as i32)
    }
    pub fn get(&self, id: i32) -> Option<&str> {
        let id = id as usize;
        if id >= self.sym.len() {
            return None;
        }
        self.sym[id].as_deref()
    }
    pub fn compile(&mut self, token: &str) {
        for line in token.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() == 2 {
                if let Ok(id) = parts[1].parse::<usize>() {
                    self.add(id as i32, parts[0]);
                }
            }
        }
    }
}
