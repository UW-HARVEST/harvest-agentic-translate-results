use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
const SYM_INIT_SIZE: usize = 1024;
const TOKEN_SIZE: usize = 1024;
type Token = String;
pub fn fnv32(token: &str) -> usize{
    const FNV_PRIME_32: u32 = 16_777_619;
    const FNV_OFFSET_32: u32 = 2_166_136_261;
    let mut hsh = FNV_OFFSET_32;
    for byte in token.as_bytes() {
        hsh ^= u32::from(*byte);
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
        let mut line = String::new();
        let mut line_no = 1usize;

        loop {
            line.clear();
            if fin.read_line(&mut line)? == 0 {
                break;
            }

            let trimmed = line.trim_end_matches(['\n', '\r']);
            let mut parts = trimmed.split('\t');
            if let (Some(token), Some(id), None) = (parts.next(), parts.next(), parts.next()) {
                let token_id: i32 = id.parse().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid input line {}: {}", line_no, trimmed),
                    )
                })?;
                self.add(token_id, token);
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid input line {}: {}", line_no, trimmed),
                ));
            }
            line_no += 1;
        }
        Ok(())
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let file = File::open(filename)?;
        let mut reader = BufReader::new(file);
        self.read(&mut reader)
    }
    pub fn print(&self) {
        let mut items: Vec<(&String, &usize)> = self.rev.iter().collect();
        items.sort_by_key(|(_, id)| **id);
        for (token, id) in items {
            println!("{token}\t{id}");
        }
    }
    pub fn add(&mut self, id: i32, token: &str) -> Option<String> {
        let id = usize::try_from(id).ok()?;

        while self.n_max <= id {
            self.n_max *= 2;
        }

        let token_owned = token.to_string();
        if !self.rev.contains_key(&token_owned) {
            self.n_items += 1;
        }
        if id < self.sym.len() {
            self.sym[id] = Some(token_owned.clone());
        }
        self.rev.insert(token_owned.clone(), id);
        Some(token_owned)
    }
    pub fn getr(&self, token: &str) -> Option<i32> {
        Some(
            self.rev
                .get(token)
                .map(|id| *id as i32)
                .unwrap_or(-1),
        )
    }
    pub fn get(&self, id: i32) -> Option<&str> {
        let id = usize::try_from(id).ok()?;
        self.sym
            .get(id)
            .and_then(|token| token.as_deref())
            .or_else(|| {
                self.rev
                    .iter()
                    .find_map(|(token, token_id)| (*token_id == id).then_some(token.as_str()))
            })
    }
    pub fn compile(&mut self, token: &str) {
        let _ = self.add(self.n_items as i32, token);
    }
}
