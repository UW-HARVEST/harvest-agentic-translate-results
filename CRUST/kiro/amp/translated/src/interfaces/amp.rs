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

impl Amp {
    /// Decodes the given buffer into this message.
    ///
    /// # Arguments
    ///
    /// * `buf` - A string slice containing the encoded message.
    pub fn decode(&mut self, buf: &str) {
        let b = buf.as_bytes()[0];
        self.version = (b >> 4) as i16;
        self.argc = (b & 0xf) as i16;
        // Store: 4-byte position (initially 0) + original data after header
        self.buf = String::new();
        // Position = 0, encoded as 4 chars
        self.buf.push(0 as char);
        self.buf.push(0 as char);
        self.buf.push(0 as char);
        self.buf.push(0 as char);
        self.buf.push_str(&buf[1..]);
    }

    /// Decodes and returns the next argument from the message.
    ///
    /// # Returns
    ///
    /// A string slice representing the next decoded argument.
    pub fn decode_arg(&mut self) -> &str {
        let bytes = self.buf.as_bytes();
        // Read current position from first 4 bytes
        let pos = ((bytes[0] as usize) << 24
            | (bytes[1] as usize) << 16
            | (bytes[2] as usize) << 8
            | (bytes[3] as usize)) + 4; // +4 to skip position header

        // Read arg length
        let len = ((bytes[pos] as u32) << 24
            | (bytes[pos + 1] as u32) << 16
            | (bytes[pos + 2] as u32) << 8
            | (bytes[pos + 3] as u32)) as usize;

        let arg_start = pos + 4;
        let arg_end = arg_start + len;

        // Update position: new_pos = old_pos_without_header + 4 + len
        let new_pos = (arg_end - 4) as u32; // subtract the 4-byte position header
        // We need to update the first 4 bytes
        // Since String doesn't allow byte-level mutation easily, rebuild
        let arg_slice_start = arg_start;
        let arg_slice_end = arg_end;

        // Rebuild: new_pos(4 bytes) + same data
        let data = self.buf[4..].to_string();
        self.buf.clear();
        self.buf.push(((new_pos >> 24) & 0xff) as u8 as char);
        self.buf.push(((new_pos >> 16) & 0xff) as u8 as char);
        self.buf.push(((new_pos >> 8) & 0xff) as u8 as char);
        self.buf.push((new_pos & 0xff) as u8 as char);
        self.buf.push_str(&data);

        // Return slice of the arg within self.buf
        &self.buf[arg_slice_start..arg_slice_end]
    }
}

/// Encodes the given arguments into a message buffer.
///
/// # Arguments
///
/// * `argv` - A slice of string slices representing the arguments.
///
/// # Returns
///
/// A `String` containing the encoded message.
pub fn amp_encode(argv: &[&str]) -> String {
    let argc = argv.len();
    let mut out = String::new();
    out.push(((AMP_VERSION << 4) as u8 | argc as u8) as char);
    for arg in argv {
        let len = arg.len() as u32;
        out.push((len >> 24 & 0xff) as u8 as char);
        out.push((len >> 16 & 0xff) as u8 as char);
        out.push((len >> 8 & 0xff) as u8 as char);
        out.push((len & 0xff) as u8 as char);
        out.push_str(arg);
    }
    out
}
