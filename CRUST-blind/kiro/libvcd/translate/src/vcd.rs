use std::fs::File;
use std::io::{BufReader, Read};

pub fn isexpression(c: char) -> bool {
    match c {
        '-' | '0'..='9' | 'z' | 'Z' | 'x' | 'X' | 'b' | 'U' => true,
        _ => false,
    }
}
pub const VCD_SIGNAL_COUNT: usize = 32;
pub const VCD_VALUE_CHANGE_COUNT: usize = 4096;
pub const VCD_SIGNAL_SIZE: usize = 64;
pub const VCD_NAME_SIZE: usize = 32;
pub const VCD_TIME_UNIT_SIZE: usize = 8;
pub const VCD_VERSION_SIZE: usize = 64;
pub const VCD_DATE_SIZE: usize = 64;
pub type Timestamp = u32;
#[derive(Debug, Clone, Copy)]
pub struct ValueChange {
    pub timestamp: Timestamp,
    pub value: [u8; VCD_SIGNAL_SIZE],
}
#[derive(Debug)]
pub struct Signal {
    pub name: [u8; VCD_NAME_SIZE],
    pub size: usize,
    pub value_changes: Vec<ValueChange>,
}
#[derive(Debug)]
pub struct Timescale {
    pub unit: [u8; VCD_TIME_UNIT_SIZE],
    pub scale: usize,
}
#[derive(Debug)]
pub struct VCD {
    pub signals: Vec<Signal>,
    pub date: [u8; VCD_DATE_SIZE],
    pub version: [u8; VCD_VERSION_SIZE],
    pub timescale: Timescale,
}

fn copy_to_fixed<const N: usize>(dest: &mut [u8; N], src: &str) {
    let bytes = src.as_bytes();
    let len = bytes.len().min(N - 1);
    dest[..len].copy_from_slice(&bytes[..len]);
}

fn skip_whitespace(s: &str, pos: usize) -> usize {
    let mut p = pos;
    while p < s.len() && s.as_bytes()[p].is_ascii_whitespace() {
        p += 1;
    }
    p
}

fn read_word(s: &str, pos: usize) -> Option<(&str, usize)> {
    let start = skip_whitespace(s, pos);
    if start >= s.len() {
        return None;
    }
    let mut end = start;
    while end < s.len() && !s.as_bytes()[end].is_ascii_whitespace() {
        end += 1;
    }
    Some((&s[start..end], end))
}

fn find_dollar(s: &str, pos: usize) -> usize {
    s[pos..].find('$').map(|i| pos + i).unwrap_or(s.len())
}

fn find_newline(s: &str, pos: usize) -> usize {
    s[pos..].find('\n').map(|i| pos + i).unwrap_or(s.len())
}

fn parse_instruction_at(s: &str, pos: usize, vcd: &mut VCD, state: &mut State) -> Option<usize> {
    let (instruction, mut p) = read_word(s, pos)?;

    if instruction == "end" || instruction == "dumpvars" || instruction == "dumpall" {
        return Some(p);
    }

    if instruction == "scope" {
        match *state {
            State::BeforeModuleDefinitions => *state = State::InsideTopModule,
            State::InsideTopModule => *state = State::InsideInnerModules,
            _ => {}
        }
        return Some(find_dollar(s, p));
    }

    if instruction == "upscope" || instruction == "enddefinitions" || instruction == "comment" {
        return Some(find_dollar(s, p));
    }

    if instruction == "var" {
        if *state == State::InsideInnerModules {
            let nl = find_newline(s, p);
            return Some(if nl < s.len() { nl + 1 } else { nl });
        }

        // Skip type
        let (_, adv) = read_word(s, p)?;
        p = adv;
        // Read size
        let (size_str, adv) = read_word(s, p)?;
        let size: usize = size_str.parse().ok()?;
        p = adv;
        // Read signal_id
        let (signal_id_str, adv) = read_word(s, p)?;
        let signal_id = signal_id_str.to_string();
        p = adv;
        // Read name (up to space or $)
        let start = skip_whitespace(s, p);
        let mut end = start;
        while end < s.len() {
            let c = s.as_bytes()[end];
            if c == b'$' || c == b' ' {
                break;
            }
            end += 1;
        }
        let name = &s[start..end];
        p = end;

        let index = get_signal_index(&signal_id);
        let is_alias = if let Some(idx) = index {
            idx < vcd.signals.len() && vcd.signals[idx].size != 0
        } else {
            false
        };

        if !is_alias {
            let mut sig = Signal {
                name: [0u8; VCD_NAME_SIZE],
                size,
                value_changes: Vec::new(),
            };
            copy_to_fixed(&mut sig.name, name);
            vcd.signals.push(sig);
        }

        return Some(find_dollar(s, p));
    }

    if instruction == "date" {
        let nl = find_newline(s, p);
        if nl < s.len() {
            let after_nl = nl + 1;
            let end = s[after_nl..].find(|c: char| c == '$' || c == '\n')
                .map(|i| after_nl + i).unwrap_or(s.len());
            let trimmed = s[after_nl..end].trim_start_matches('\t');
            copy_to_fixed(&mut vcd.date, trimmed);
            return Some(end);
        }
        return Some(p);
    }

    if instruction == "version" {
        let nl = find_newline(s, p);
        if nl < s.len() {
            let after_nl = nl + 1;
            let end = s[after_nl..].find(|c: char| c == '$' || c == '\n')
                .map(|i| after_nl + i).unwrap_or(s.len());
            let trimmed = s[after_nl..end].trim_start_matches('\t');
            copy_to_fixed(&mut vcd.version, trimmed);
            return Some(end);
        }
        return Some(p);
    }

    if instruction == "timescale" {
        let nl = find_newline(s, p);
        if nl < s.len() {
            let after_nl = nl + 1;
            let content_start = skip_whitespace(s, after_nl);
            // Parse digits for scale
            let mut digit_end = content_start;
            while digit_end < s.len() && s.as_bytes()[digit_end].is_ascii_digit() {
                digit_end += 1;
            }
            if digit_end > content_start {
                vcd.timescale.scale = s[content_start..digit_end].parse().unwrap_or(0);
            }
            // Unit: rest until $ or newline
            let unit_end = s[digit_end..].find(|c: char| c == '$' || c == '\n')
                .map(|i| digit_end + i).unwrap_or(s.len());
            copy_to_fixed(&mut vcd.timescale.unit, &s[digit_end..unit_end]);
            return Some(unit_end);
        }
        return Some(p);
    }

    None
}

fn parse_timestamp_at(s: &str, pos: usize) -> Option<(Timestamp, usize)> {
    let mut end = pos;
    while end < s.len() && s.as_bytes()[end].is_ascii_digit() {
        end += 1;
    }
    if end == pos {
        return None;
    }
    let ts: Timestamp = s[pos..end].parse().ok()?;
    Some((ts, end))
}

fn parse_assignment_at(s: &str, pos: usize, vcd: &mut VCD, timestamp: Timestamp) -> Option<usize> {
    let line_end = find_newline(s, pos);
    let buffer = &s[pos..line_end];

    if buffer.is_empty() {
        return None;
    }

    let first = buffer.as_bytes()[0] as char;
    let is_vector = !matches!(first, '0' | '1' | 'x' | 'X' | 'z' | 'Z');

    let (value, signal_id) = if is_vector {
        let parts: Vec<&str> = buffer.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return None;
        }
        (parts[0], parts[1].trim())
    } else {
        (&buffer[..1], &buffer[1..])
    };

    let consumed = if line_end < s.len() { line_end + 1 } else { line_end };

    if signal_id.len() > 1 {
        return Some(consumed);
    }

    let index = match get_signal_index(signal_id) {
        Some(idx) => idx,
        None => return Some(consumed),
    };

    if index >= vcd.signals.len() {
        return Some(consumed);
    }

    let mut vc = ValueChange {
        timestamp,
        value: [0u8; VCD_SIGNAL_SIZE],
    };
    let val_bytes = value.as_bytes();
    let len = val_bytes.len().min(VCD_SIGNAL_SIZE - 1);
    vc.value[..len].copy_from_slice(&val_bytes[..len]);
    vcd.signals[index].value_changes.push(vc);

    Some(consumed)
}

impl VCD {
    pub fn read_from_path(path: &str) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut content = String::new();
        reader.read_to_string(&mut content)?;

        let mut vcd = VCD {
            signals: Vec::new(),
            date: [0u8; VCD_DATE_SIZE],
            version: [0u8; VCD_VERSION_SIZE],
            timescale: Timescale {
                unit: [0u8; VCD_TIME_UNIT_SIZE],
                scale: 0,
            },
        };
        let mut current_timestamp: Timestamp = 0;
        let mut state = State::BeforeModuleDefinitions;
        let mut pos = 0;

        while pos < content.len() {
            let ch = content.as_bytes()[pos] as char;

            if ch == '$' {
                pos += 1;
                match parse_instruction_at(&content, pos, &mut vcd, &mut state) {
                    Some(new_pos) => { pos = new_pos; continue; }
                    None => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "parse error")),
                }
            } else if ch == '#' {
                pos += 1;
                match parse_timestamp_at(&content, pos) {
                    Some((ts, new_pos)) => { current_timestamp = ts; pos = new_pos; continue; }
                    None => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "parse error")),
                }
            } else if isexpression(ch) {
                match parse_assignment_at(&content, pos, &mut vcd, current_timestamp) {
                    Some(new_pos) => { pos = new_pos; continue; }
                    None => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "parse error")),
                }
            } else if ch.is_whitespace() {
                pos += 1;
                continue;
            } else {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "parse error"));
            }
        }

        Ok(vcd)
    }

    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        let name_bytes = signal_name.as_bytes();
        self.signals.iter().find(|s| {
            let len = s.name.iter().position(|&b| b == 0).unwrap_or(s.name.len());
            &s.name[..len] == name_bytes
        })
    }
}

impl Signal {
    pub fn get_value_at_timestamp(&self, timestamp: Timestamp) -> Option<&[u8; VCD_SIGNAL_SIZE]> {
        let mut previous: Option<&[u8; VCD_SIGNAL_SIZE]> = None;
        for vc in &self.value_changes {
            if timestamp < vc.timestamp {
                break;
            }
            previous = Some(&vc.value);
        }
        previous
    }
}

pub const BUFFER_LENGTH: usize = 512;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    BeforeModuleDefinitions,
    InsideTopModule,
    InsideInnerModules,
}

pub fn parse_instruction(
    file: &File,
    vcd: &mut VCD,
    state: &mut State,
) -> Result<(), std::io::Error> {
    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    parse_instruction_at(&content, 0, vcd, state)
        .map(|_| ())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "parse error"))
}

pub fn parse_timestamp(file: &File) -> Result<Timestamp, std::io::Error> {
    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    parse_timestamp_at(&content, 0)
        .map(|(ts, _)| ts)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "parse error"))
}

pub fn parse_assignment(
    file: &File,
    vcd: &mut VCD,
    timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    parse_assignment_at(&content, 0, vcd, *timestamp)
        .map(|_| ())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "parse error"))
}

pub fn get_signal_index(s: &str) -> Option<usize> {
    let first = s.bytes().next()?;
    let id = first as i32 - b'!' as i32;
    if id < 0 || id as usize >= VCD_SIGNAL_COUNT {
        return None;
    }
    Some(id as usize)
}
