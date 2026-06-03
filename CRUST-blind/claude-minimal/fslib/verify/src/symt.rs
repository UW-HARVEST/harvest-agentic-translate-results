use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
const SYM_INIT_SIZE: usize = 1024;
const TOKEN_SIZE: usize = 1024;
const FNV_PRIME_32: u32 = 16777619;
const FNV_OFFSET_32: u32 = 2166136261;

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
        for (i, s) in self.sym.iter().enumerate() {
            if let Some(token) = s {
                self.rev.insert(token.clone(), i);
            }
        }
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
            let trimmed = line.trim_end_matches(|c| c == '\n' || c == '\r');
            let mut parts = trimmed.split('\t');
            let token = parts.next();
            let id_str = parts.next();
            match (token, id_str) {
                (Some(token), Some(id_str)) => {
                    let id: usize = id_str.trim().parse().map_err(|_| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Invalid input line {}: {}", line_no, line),
                        )
                    })?;
                    self.add(id as i32, token);
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid input line {}: {}", line_no, line),
                    ));
                }
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
        let prev = self.sym[id].take();
        self.sym[id] = Some(token.to_string());
        self.n_items += 1;
        // keep reverse mapping consistent if it was built
        if !self.rev.is_empty() {
            self.rev.insert(token.to_string(), id);
        }
        prev
    }
    pub fn getr(&self, token: &str) -> Option<i32> {
        self.rev.get(token).map(|v| *v as i32)
    }
    pub fn get(&self, id: i32) -> Option<&str> {
        let id = id as usize;
        if id >= self.sym.len() {
            return None;
        }
        self.sym[id].as_deref()
    }
    pub fn compile(&mut self, token: &str) {
        // parse multi-line "<token>\t<id>" content
        for (line_no, line) in token.lines().enumerate() {
            let mut parts = line.split('\t');
            let tok = parts.next();
            let id_str = parts.next();
            if let (Some(tok), Some(id_str)) = (tok, id_str) {
                if let Ok(id) = id_str.trim().parse::<usize>() {
                    self.add(id as i32, tok);
                    continue;
                }
            }
            eprintln!("Invalid input line {}: {}", line_no + 1, line);
        }
    }
}

#[allow(dead_code)]
const _TOKEN_SIZE_UNUSED: usize = TOKEN_SIZE;
