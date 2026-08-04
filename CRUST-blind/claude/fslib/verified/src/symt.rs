use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
const SYM_INIT_SIZE: usize = 1024;
const TOKEN_SIZE: usize = 1024;
type Token = String;

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
        // Drop self
    }
    pub fn reverse(&self) -> &HashMap<String, usize> {
        &self.rev
    }
    pub fn read(&mut self, fin: &mut dyn BufRead) -> io::Result<()> {
        let mut line_no: usize = 1;
        let mut buf = String::new();
        loop {
            buf.clear();
            let n = fin.read_line(&mut buf)?;
            if n == 0 {
                break;
            }
            // parse "<token>\t<id>"
            let trimmed = buf.trim_end_matches(|c| c == '\n' || c == '\r');
            // tokenize on whitespace; first token is symbol, second is id
            let mut parts = trimmed.split_whitespace();
            let token = parts.next();
            let id_str = parts.next();
            match (token, id_str) {
                (Some(tok), Some(idstr)) => match idstr.parse::<usize>() {
                    Ok(token_id) => {
                        self.add(token_id as i32, tok);
                    }
                    Err(_) => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Invalid input line {}: {}", line_no, buf),
                        ));
                    }
                },
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid input line {}: {}", line_no, buf),
                    ));
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
        for (i, v) in self.sym.iter().enumerate() {
            if let Some(s) = v {
                println!("{}\t{}", s, i);
            }
        }
    }
    pub fn add(&mut self, id: i32, token: &str) -> Option<String> {
        if id < 0 {
            return None;
        }
        let id = id as usize;
        let s = self.n_max;
        while self.n_max <= id {
            self.n_max *= 2;
        }
        if self.n_max > s {
            self.sym.resize(self.n_max, None);
        }
        let t: Token = token.to_string();
        self.sym[id] = Some(t.clone());
        self.rev.insert(t.clone(), id);
        self.n_items += 1;
        Some(t)
    }
    pub fn getr(&self, token: &str) -> Option<i32> {
        // search reverse index, but if not present, also rebuild from sym
        if let Some(v) = self.rev.get(token) {
            return Some(*v as i32);
        }
        // Fallback: linear scan
        for (i, v) in self.sym.iter().enumerate() {
            if let Some(s) = v {
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
        let id = id as usize;
        if id >= self.sym.len() {
            return None;
        }
        self.sym[id].as_deref()
    }
    pub fn compile(&mut self, token: &str) {
        let mut line_no = 1usize;
        for line in token.split('\n') {
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split_whitespace();
            let tok = parts.next();
            let idstr = parts.next();
            match (tok, idstr) {
                (Some(t), Some(idstr)) => {
                    if let Ok(token_id) = idstr.parse::<usize>() {
                        self.add(token_id as i32, t);
                    }
                }
                _ => {}
            }
            line_no += 1;
        }
        let _ = line_no;
        let _ = TOKEN_SIZE;
    }
}
