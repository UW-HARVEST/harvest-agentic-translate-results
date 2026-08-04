use std::fs::File;
use std::io::Read;

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
#[derive(Debug, Clone)]
pub struct ValueChange {
    pub timestamp: Timestamp,
    pub value: Vec<u8>,
}
#[derive(Debug)]
pub struct Signal {
    pub name: Vec<u8>,
    pub size: usize,
    pub value_changes: Vec<ValueChange>,
}
#[derive(Debug)]
pub struct Timescale {
    pub unit: Vec<u8>,
    pub scale: usize,
}
#[derive(Debug)]
pub struct VCD {
    pub signals: Vec<Signal>,
    pub date: Vec<u8>,
    pub version: Vec<u8>,
    pub timescale: Timescale,
}

impl VCD {
    pub fn read_from_path(path: &str) -> Result<Self, std::io::Error> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let mut vcd = VCD {
            signals: Vec::new(),
            date: Vec::new(),
            version: Vec::new(),
            timescale: Timescale {
                unit: Vec::new(),
                scale: 0,
            },
        };
        for _ in 0..VCD_SIGNAL_COUNT {
            vcd.signals.push(Signal {
                name: Vec::new(),
                size: 0,
                value_changes: Vec::new(),
            });
        }

        let mut state = State::BeforeModuleDefinitions;
        let mut current_timestamp: Timestamp = 0;
        let mut signals_count: usize = 0;
        let bytes = content.as_bytes();
        let mut pos = 0;

        while pos < bytes.len() {
            let c = bytes[pos] as char;

            if c == '$' {
                pos += 1;
                while pos < bytes.len() && (bytes[pos] as char).is_ascii_whitespace() {
                    pos += 1;
                }
                let start = pos;
                while pos < bytes.len() && !(bytes[pos] as char).is_ascii_whitespace() {
                    pos += 1;
                }
                let instruction = &content[start..pos];

                if instruction == "end" || instruction == "dumpvars" || instruction == "dumpall" {
                    continue;
                }
                if instruction == "scope" {
                    match state {
                        State::BeforeModuleDefinitions => state = State::InsideTopModule,
                        State::InsideTopModule => state = State::InsideInnerModules,
                        _ => {}
                    }
                    while pos < bytes.len() && bytes[pos] != b'$' {
                        pos += 1;
                    }
                    continue;
                }
                if instruction == "upscope" || instruction == "enddefinitions" || instruction == "comment" {
                    while pos < bytes.len() && bytes[pos] != b'$' {
                        pos += 1;
                    }
                    continue;
                }
                if instruction == "var" {
                    if state == State::InsideInnerModules {
                        while pos < bytes.len() && bytes[pos] != b'\n' {
                            pos += 1;
                        }
                        continue;
                    }
                    while pos < bytes.len() && (bytes[pos] as char).is_ascii_whitespace() {
                        pos += 1;
                    }
                    while pos < bytes.len() && !(bytes[pos] as char).is_ascii_whitespace() {
                        pos += 1;
                    }
                    while pos < bytes.len() && (bytes[pos] as char).is_ascii_whitespace() {
                        pos += 1;
                    }
                    let size_start = pos;
                    while pos < bytes.len() && (bytes[pos] as char).is_ascii_digit() {
                        pos += 1;
                    }
                    let size: usize = content[size_start..pos].parse().unwrap_or(0);
                    while pos < bytes.len() && (bytes[pos] as char).is_ascii_whitespace() {
                        pos += 1;
                    }
                    let id_start = pos;
                    while pos < bytes.len() && !(bytes[pos] as char).is_ascii_whitespace() {
                        pos += 1;
                    }
                    let _signal_id = &content[id_start..pos];
                    while pos < bytes.len() && (bytes[pos] as char).is_ascii_whitespace() {
                        pos += 1;
                    }
                    let name_start = pos;
                    while pos < bytes.len() && bytes[pos] != b'$' && bytes[pos] != b'\n' {
                        pos += 1;
                    }
                    let name = content[name_start..pos].trim_end();

                    let signal = &mut vcd.signals[signals_count];
                    signals_count += 1;
                    signal.name = name.as_bytes().to_vec();
                    signal.size = size;
                    continue;
                }
                if instruction == "date" {
                    if pos < bytes.len() && bytes[pos] == b'\n' {
                        pos += 1;
                    }
                    if pos < bytes.len() && bytes[pos] == b'\t' {
                        pos += 1;
                    }
                    let start = pos;
                    while pos < bytes.len() && bytes[pos] != b'$' && bytes[pos] != b'\n' {
                        pos += 1;
                    }
                    vcd.date = content[start..pos].as_bytes().to_vec();
                    continue;
                }
                if instruction == "version" {
                    if pos < bytes.len() && bytes[pos] == b'\n' {
                        pos += 1;
                    }
                    if pos < bytes.len() && bytes[pos] == b'\t' {
                        pos += 1;
                    }
                    let start = pos;
                    while pos < bytes.len() && bytes[pos] != b'$' && bytes[pos] != b'\n' {
                        pos += 1;
                    }
                    vcd.version = content[start..pos].as_bytes().to_vec();
                    continue;
                }
                if instruction == "timescale" {
                    if pos < bytes.len() && bytes[pos] == b'\n' {
                        pos += 1;
                    }
                    if pos < bytes.len() && bytes[pos] == b'\t' {
                        pos += 1;
                    }
                    let scale_start = pos;
                    while pos < bytes.len() && (bytes[pos] as char).is_ascii_digit() {
                        pos += 1;
                    }
                    vcd.timescale.scale = content[scale_start..pos].parse().unwrap_or(0);
                    let unit_start = pos;
                    while pos < bytes.len() && bytes[pos] != b'$' && bytes[pos] != b'\n' {
                        pos += 1;
                    }
                    vcd.timescale.unit = content[unit_start..pos].as_bytes().to_vec();
                    continue;
                }
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "unknown instruction"));
            } else if c == '#' {
                pos += 1;
                let ts_start = pos;
                while pos < bytes.len() && (bytes[pos] as char).is_ascii_digit() {
                    pos += 1;
                }
                if ts_start == pos {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "bad timestamp"));
                }
                current_timestamp = content[ts_start..pos].parse().unwrap_or(0);
            } else if isexpression(c) {
                let line_start = pos;
                while pos < bytes.len() && bytes[pos] != b'\n' {
                    pos += 1;
                }
                let line = &content[line_start..pos];
                let first_char = line.as_bytes()[0] as char;
                let is_vector = !"01xXzZ".contains(first_char);

                let parts = if is_vector {
                    if let Some(sp) = line.find(' ') {
                        let mut val = &line[..sp];
                        // Strip 'b'/'B' prefix for binary values
                        if val.starts_with('b') || val.starts_with('B') {
                            val = &val[1..];
                        }
                        Some((val, &line[sp + 1..]))
                    } else {
                        None
                    }
                } else {
                    Some((&line[..1], &line[1..]))
                };

                if let Some((value, signal_id)) = parts {
                    if signal_id.len() > 1 {
                        continue;
                    }
                    if let Some(index) = get_signal_index(signal_id) {
                        if index < signals_count {
                            vcd.signals[index].value_changes.push(ValueChange {
                                timestamp: current_timestamp,
                                value: value.as_bytes().to_vec(),
                            });
                        }
                    }
                }
            } else if c.is_ascii_whitespace() {
                pos += 1;
            } else {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "unexpected char"));
            }
        }

        vcd.signals.truncate(signals_count);
        Ok(vcd)
    }

    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        self.signals.iter().find(|s| s.name == signal_name.as_bytes())
    }
}

impl Signal {
    pub fn get_value_at_timestamp(&self, timestamp: Timestamp) -> Option<&Vec<u8>> {
        let mut previous: Option<&Vec<u8>> = None;
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
    _file: &File,
    _vcd: &mut VCD,
    _state: &mut State,
) -> Result<(), std::io::Error> {
    Ok(())
}
pub fn parse_timestamp(_file: &File) -> Result<Timestamp, std::io::Error> {
    Ok(0)
}
pub fn parse_assignment(
    _file: &File,
    _vcd: &mut VCD,
    _timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    Ok(())
}
pub fn get_signal_index(s: &str) -> Option<usize> {
    let first = s.bytes().next()?;
    let id = (first as i32) - (b'!' as i32);
    if id < 0 || id as usize >= VCD_SIGNAL_COUNT {
        None
    } else {
        Some(id as usize)
    }
}
