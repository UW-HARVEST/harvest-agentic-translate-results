/// Protocol version.
pub const AMP_VERSION: i16 = 1;
/// Message struct.
pub struct Amp {
    /// Protocol version.
    pub version: i16,
    /// Number of arguments.
    pub argc: i16,
    /// Encoded buffer.
    pub buf: String,
}

fn bytes_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

fn string_to_bytes(s: &str) -> Vec<u8> {
    s.chars().map(|c| c as u8).collect()
}

impl Amp {
    /// Decodes the given buffer into this message.
    pub fn decode(&mut self, buf: &str) {
        let header = buf.chars().next().unwrap() as u8;
        self.version = (header >> 4) as i16;
        self.argc = (header & 0xf) as i16;
        self.buf = buf.chars().skip(1).collect();
    }
    /// Decodes and returns the next argument from the message.
    pub fn decode_arg(&mut self) -> &str {
        let bytes = string_to_bytes(&self.buf);
        let len = ((bytes[0] as u32) << 24
            | (bytes[1] as u32) << 16
            | (bytes[2] as u32) << 8
            | bytes[3] as u32) as usize;
        let arg_end = 4 + len;
        let remaining = bytes_to_string(&bytes[arg_end..]);
        let arg = bytes_to_string(&bytes[4..arg_end]);
        let rem_len = remaining.len();
        self.buf = remaining + &arg;
        &self.buf[rem_len..]
    }
}
/// Encodes the given arguments into a message buffer.
pub fn amp_encode(argv: &[&str]) -> String {
    let argc = argv.len();
    let mut result: Vec<u8> = Vec::with_capacity(1 + argc * 4);
    result.push((AMP_VERSION as u8) << 4 | argc as u8);
    for arg in argv {
        let bytes = arg.as_bytes();
        let len = bytes.len() as u32;
        result.push((len >> 24) as u8);
        result.push((len >> 16) as u8);
        result.push((len >> 8) as u8);
        result.push(len as u8);
        result.extend_from_slice(bytes);
    }
    bytes_to_string(&result)
}
