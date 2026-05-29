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

impl Default for VCD {
    fn default() -> Self {
        Self {
            signals: Vec::new(),
            date: Vec::new(),
            version: Vec::new(),
            timescale: Timescale {
                unit: Vec::new(),
                scale: 0,
            },
        }
    }
}

impl VCD {
    pub fn read_from_path(path: &str) -> Result<Self, std::io::Error> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let mut vcd = VCD::default();
        let mut current_timestamp: Timestamp = 0;
        let mut state = State::BeforeModuleDefinitions;

        let bytes = content.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if c == '$' {
                i += 1;
                if let Some(new_i) = parse_instruction_str(bytes, i, &mut vcd, &mut state) {
                    i = new_i;
                    continue;
                }
            } else if c == '#' {
                i += 1;
                if let Some((new_i, ts)) = parse_timestamp_str(bytes, i) {
                    current_timestamp = ts;
                    i = new_i;
                    continue;
                }
            } else if isexpression(c) {
                if let Some(new_i) =
                    parse_assignment_str(bytes, i, &mut vcd, current_timestamp)
                {
                    i = new_i;
                    continue;
                }
            } else if c.is_whitespace() {
                i += 1;
                continue;
            }

            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to parse VCD",
            ));
        }

        Ok(vcd)
    }

    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        let target = signal_name.as_bytes();
        for signal in &self.signals {
            if signal.name == target {
                return Some(signal);
            }
        }
        None
    }
}

impl Signal {
    pub fn get_value_at_timestamp(&self, timestamp: Timestamp) -> Option<&Vec<u8>> {
        let mut previous_value: Option<&Vec<u8>> = None;
        for value_change in &self.value_changes {
            if timestamp < value_change.timestamp {
                break;
            }
            previous_value = Some(&value_change.value);
        }
        previous_value
    }
}

pub const BUFFER_LENGTH: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    BeforeModuleDefinitions,
    InsideTopModule,
    InsideInnerModules,
}

#[allow(private_interfaces)]
pub fn parse_instruction(
    file: &File,
    vcd: &mut VCD,
    state: &mut State,
) -> Result<(), std::io::Error> {
    // Read entire remaining content of the file and parse one instruction.
    let mut file_clone = file.try_clone()?;
    let mut content = String::new();
    file_clone.read_to_string(&mut content)?;
    let bytes = content.as_bytes();
    if parse_instruction_str(bytes, 0, vcd, state).is_some() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Failed to parse instruction",
        ))
    }
}

pub fn parse_timestamp(file: &File) -> Result<Timestamp, std::io::Error> {
    let mut file_clone = file.try_clone()?;
    let mut content = String::new();
    file_clone.read_to_string(&mut content)?;
    let bytes = content.as_bytes();
    parse_timestamp_str(bytes, 0)
        .map(|(_, ts)| ts)
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to parse timestamp")
        })
}

pub fn parse_assignment(
    file: &File,
    vcd: &mut VCD,
    timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    let mut file_clone = file.try_clone()?;
    let mut content = String::new();
    file_clone.read_to_string(&mut content)?;
    let bytes = content.as_bytes();
    if parse_assignment_str(bytes, 0, vcd, *timestamp).is_some() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Failed to parse assignment",
        ))
    }
}

pub fn get_signal_index(s: &str) -> Option<usize> {
    let first = s.as_bytes().first()?;
    let id = (*first as i32) - (b'!' as i32);
    if id < 0 || id >= VCD_SIGNAL_COUNT as i32 {
        return None;
    }
    Some(id as usize)
}

// Helper functions for parsing using a byte slice with a position index.

fn skip_spaces_and_tabs(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    i
}

fn skip_whitespace(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    i
}

fn read_until_dollar(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i] != b'$' {
        i += 1;
    }
    i
}

fn read_word(bytes: &[u8], mut i: usize) -> (usize, &[u8]) {
    // Skip leading whitespace
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    let start = i;
    while i < bytes.len() && !(bytes[i] as char).is_whitespace() {
        i += 1;
    }
    (i, &bytes[start..i])
}

fn parse_instruction_str(
    bytes: &[u8],
    i: usize,
    vcd: &mut VCD,
    state: &mut State,
) -> Option<usize> {
    // Read instruction word.
    let (mut idx, instruction) = read_word(bytes, i);
    if instruction.is_empty() {
        return None;
    }

    let instr = instruction;

    if instr == b"end" || instr == b"dumpvars" || instr == b"dumpall" {
        return Some(idx);
    }

    if instr == b"scope" {
        match *state {
            State::BeforeModuleDefinitions => *state = State::InsideTopModule,
            State::InsideTopModule => *state = State::InsideInnerModules,
            _ => {}
        }
        // fscanf(file, "\n%*[^$]") - skip a newline then everything until $
        // We just skip until $.
        idx = read_until_dollar(bytes, idx);
        return Some(idx);
    }

    if instr == b"upscope" || instr == b"enddefinitions" || instr == b"comment" {
        idx = read_until_dollar(bytes, idx);
        return Some(idx);
    }

    if instr == b"var" {
        if *state == State::InsideInnerModules {
            // Skip rest of line.
            while idx < bytes.len() && bytes[idx] != b'\n' {
                idx += 1;
            }
            if idx < bytes.len() {
                idx += 1; // consume newline
            }
            return Some(idx);
        }

        // Parse: " %*s %zu %[^ ] %[^ $]%*[^$]"
        // Skip whitespace, skip type word, then size, then signal_id, then name (up to $).
        // Skip whitespace
        idx = skip_whitespace(bytes, idx);
        // Skip type word (e.g. "wire", "reg")
        let (after_type, _type_word) = read_word(bytes, idx);
        idx = after_type;
        // Skip whitespace
        idx = skip_whitespace(bytes, idx);
        // Read size as decimal number
        let size_start = idx;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }
        let size_str = std::str::from_utf8(&bytes[size_start..idx]).ok()?;
        let size: usize = size_str.parse().ok()?;

        // Skip whitespace
        idx = skip_whitespace(bytes, idx);
        // Read signal_id (until space)
        let id_start = idx;
        while idx < bytes.len() && bytes[idx] != b' ' && bytes[idx] != b'\t' && bytes[idx] != b'\n'
        {
            idx += 1;
        }
        let signal_id = &bytes[id_start..idx];

        // Skip spaces (just spaces and tabs, NOT newlines)
        idx = skip_spaces_and_tabs(bytes, idx);
        // Read name (until $); but trim trailing whitespace/newlines.
        let name_start = idx;
        while idx < bytes.len() && bytes[idx] != b'$' {
            idx += 1;
        }
        let mut name_end = idx;
        // Trim trailing whitespace from name
        while name_end > name_start
            && (bytes[name_end - 1] == b' '
                || bytes[name_end - 1] == b'\t'
                || bytes[name_end - 1] == b'\n'
                || bytes[name_end - 1] == b'\r')
        {
            name_end -= 1;
        }
        let name_bytes = &bytes[name_start..name_end];

        // Place into signals list at position equal to signal_id index.
        let signal_id_str = std::str::from_utf8(signal_id).ok()?;
        let index = match get_signal_index(signal_id_str) {
            Some(i) => i,
            None => {
                // No valid index; skip
                return Some(idx);
            }
        };

        // Ensure the signals vec is large enough
        while vcd.signals.len() <= index {
            vcd.signals.push(Signal {
                name: Vec::new(),
                size: 0,
                value_changes: Vec::new(),
            });
        }

        // If this signal is an alias (size != 0), keep existing.
        if vcd.signals[index].size != 0 {
            return Some(idx);
        }

        vcd.signals[index].name = name_bytes.to_vec();
        vcd.signals[index].size = size;

        return Some(idx);
    }

    if instr == b"date" {
        // fscanf(file, "\n%[^$\n]", vcd->date);
        // Match a newline then read until $ or newline.
        let mut j = idx;
        // Skip whitespace (matches \n in C scanf, also tabs, spaces)
        while j < bytes.len() && (bytes[j] as char).is_whitespace() {
            j += 1;
        }
        let date_start = j;
        while j < bytes.len() && bytes[j] != b'$' && bytes[j] != b'\n' {
            j += 1;
        }
        // Trim trailing whitespace
        let mut date_end = j;
        while date_end > date_start
            && (bytes[date_end - 1] == b' '
                || bytes[date_end - 1] == b'\t'
                || bytes[date_end - 1] == b'\r')
        {
            date_end -= 1;
        }
        vcd.date = bytes[date_start..date_end].to_vec();
        return Some(j);
    }

    if instr == b"version" {
        let mut j = idx;
        while j < bytes.len() && (bytes[j] as char).is_whitespace() {
            j += 1;
        }
        let v_start = j;
        while j < bytes.len() && bytes[j] != b'$' && bytes[j] != b'\n' {
            j += 1;
        }
        let mut v_end = j;
        while v_end > v_start
            && (bytes[v_end - 1] == b' '
                || bytes[v_end - 1] == b'\t'
                || bytes[v_end - 1] == b'\r')
        {
            v_end -= 1;
        }
        vcd.version = bytes[v_start..v_end].to_vec();
        return Some(j);
    }

    if instr == b"timescale" {
        // fscanf(file, "\n\t%zu%[^$\n]", &vcd->timescale.scale, vcd->timescale.unit);
        let mut j = idx;
        // Skip leading whitespace
        while j < bytes.len() && (bytes[j] as char).is_whitespace() {
            j += 1;
        }
        // Read decimal scale
        let s_start = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if let Ok(s) = std::str::from_utf8(&bytes[s_start..j]) {
            if let Ok(v) = s.parse::<usize>() {
                vcd.timescale.scale = v;
            }
        }
        // Read unit until $ or newline
        let u_start = j;
        while j < bytes.len() && bytes[j] != b'$' && bytes[j] != b'\n' {
            j += 1;
        }
        let mut u_end = j;
        while u_end > u_start
            && (bytes[u_end - 1] == b' '
                || bytes[u_end - 1] == b'\t'
                || bytes[u_end - 1] == b'\r')
        {
            u_end -= 1;
        }
        vcd.timescale.unit = bytes[u_start..u_end].to_vec();
        return Some(j);
    }

    None
}

fn parse_timestamp_str(bytes: &[u8], i: usize) -> Option<(usize, Timestamp)> {
    let mut j = i;
    // skip whitespace
    while j < bytes.len() && (bytes[j] as char).is_whitespace() {
        j += 1;
    }
    let s_start = j;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if s_start == j {
        return None;
    }
    let s = std::str::from_utf8(&bytes[s_start..j]).ok()?;
    let ts: Timestamp = s.parse().ok()?;
    Some((j, ts))
}

fn parse_assignment_str(
    bytes: &[u8],
    i: usize,
    vcd: &mut VCD,
    timestamp: Timestamp,
) -> Option<usize> {
    // Read until newline.
    let mut j = i;
    while j < bytes.len() && bytes[j] != b'\n' {
        j += 1;
    }
    let line = &bytes[i..j];
    let next = if j < bytes.len() { j + 1 } else { j };

    if line.is_empty() {
        return Some(next);
    }

    let first = line[0];
    let is_vector = !matches!(first, b'0' | b'1' | b'x' | b'X' | b'z' | b'Z');

    let (value_bytes, signal_id_bytes) = if is_vector {
        // Format: "%[^ ] %[^\n]"  - value up to space, then signal_id (rest, trimmed).
        // Find space in line.
        let space_pos = line.iter().position(|&b| b == b' ')?;
        let value = &line[..space_pos];
        // skip spaces
        let mut k = space_pos;
        while k < line.len() && (line[k] == b' ' || line[k] == b'\t') {
            k += 1;
        }
        let id_start = k;
        // signal_id is rest of line trimmed.
        let mut id_end = line.len();
        while id_end > id_start
            && (line[id_end - 1] == b' '
                || line[id_end - 1] == b'\t'
                || line[id_end - 1] == b'\r')
        {
            id_end -= 1;
        }
        (value, &line[id_start..id_end])
    } else {
        // Format: "%1s%[^\n]" - value is 1 char, then signal_id.
        let value = &line[..1];
        let mut k = 1;
        while k < line.len() && (line[k] == b' ' || line[k] == b'\t') {
            k += 1;
        }
        let id_start = k;
        let mut id_end = line.len();
        while id_end > id_start
            && (line[id_end - 1] == b' '
                || line[id_end - 1] == b'\t'
                || line[id_end - 1] == b'\r')
        {
            id_end -= 1;
        }
        (value, &line[id_start..id_end])
    };

    if signal_id_bytes.is_empty() {
        return Some(next);
    }

    if signal_id_bytes.len() > 1 {
        return Some(next);
    }

    let signal_id_str = std::str::from_utf8(signal_id_bytes).ok()?;
    let index = match get_signal_index(signal_id_str) {
        Some(i) => i,
        None => return Some(next),
    };

    if index >= vcd.signals.len() {
        return Some(next);
    }

    // Strip leading 'b' from vector values to match test expectations.
    let value_to_store = if is_vector && !value_bytes.is_empty() && value_bytes[0] == b'b' {
        &value_bytes[1..]
    } else {
        value_bytes
    };

    vcd.signals[index].value_changes.push(ValueChange {
        timestamp,
        value: value_to_store.to_vec(),
    });

    Some(next)
}

