use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
const SYM_INIT_SIZE: usize = 1024;
const TOKEN_SIZE: usize = 1024;
type Token = String;
pub fn fnv32(token: &str) -> usize {
    const FNV_PRIME_32: u32 = 16777619;
    const FNV_OFFSET_32: u32 = 2166136261;
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
    pub sym: Vec<Option<String>>,
    pub rev: HashMap<String, usize>,
    // Internal forward map: id -> token (not exposed)
    fwd: HashMap<usize, String>,
}
impl SymTable {
    pub fn new() -> Self {
        SymTable { n_items: 0, n_max: 0, sym: Vec::new(), rev: HashMap::new(), fwd: HashMap::new() }
    }
    pub fn remove(self) { drop(self); }
    pub fn reverse(&self) -> &HashMap<String, usize> {
        let rev_ptr = &self.rev as *const HashMap<String, usize> as *mut HashMap<String, usize>;
        unsafe {
            (*rev_ptr).clear();
            for (&id, tok) in &self.fwd {
                (*rev_ptr).insert(tok.clone(), id);
            }
            &*rev_ptr
        }
    }
    pub fn read(&mut self, fin: &mut dyn BufRead) -> io::Result<()> {
        let mut line_buf = String::new();
        while fin.read_line(&mut line_buf)? > 0 {
            let parts: Vec<&str> = line_buf.trim().split('\t').collect();
            if parts.len() == 2 {
                let token = parts[0];
                let id: i32 = parts[1].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "parse error"))?;
                self.add(id, token);
            }
            line_buf.clear();
        }
        Ok(())
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let file = File::open(filename)?;
        let mut reader = BufReader::new(file);
        self.read(&mut reader)
    }
    pub fn print(&self) {
        let mut ids: Vec<usize> = self.fwd.keys().copied().collect();
        ids.sort();
        for id in ids {
            if let Some(tok) = self.fwd.get(&id) {
                println!("{}\t{}", tok, id);
            }
        }
    }
    pub fn add(&mut self, id: i32, token: &str) -> Option<String> {
        let idx = id as usize;
        self.n_items += 1;
        self.n_max = self.n_max.max(idx + 1);
        self.fwd.insert(idx, token.to_string());
        Some(token.to_string())
    }
    pub fn getr(&self, token: &str) -> Option<i32> {
        match self.rev.get(token) {
            Some(&id) => Some(id as i32),
            None => Some(-1),
        }
    }
    pub fn get(&self, id: i32) -> Option<&str> {
        self.fwd.get(&(id as usize)).map(|s| s.as_str())
    }
    pub fn compile(&mut self, _token: &str) {}
}
