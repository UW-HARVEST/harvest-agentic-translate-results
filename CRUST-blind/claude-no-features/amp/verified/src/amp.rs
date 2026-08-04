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
        let bytes = buf.as_bytes();
        // Header byte: high 4 bits = version, low 4 bits = argc.
        self.version = (bytes[0] >> 4) as i16;
        self.argc = (bytes[0] & 0x0f) as i16;
        // Maintain the invariant that `self.buf` starts with a 4-byte
        // big-endian length prefix of the "previously returned argument".
        // For the initial state (no previous argument), prepend four
        // zero bytes so that the first `decode_arg` call skips exactly
        // these four bytes and lands on the first real length prefix.
        let mut new_buf: Vec<u8> = Vec::with_capacity(bytes.len() + 3);
        new_buf.extend_from_slice(&[0u8, 0u8, 0u8, 0u8]);
        new_buf.extend_from_slice(&bytes[1..]);
        self.buf = String::from_utf8(new_buf).expect("AMP buffer must be valid UTF-8");
    }
    /// Decodes and returns the next argument from the message.
    ///
    /// # Returns
    ///
    /// A string slice representing the next decoded argument.
    pub fn decode_arg(&mut self) -> &str {
        // Skip past the previously returned argument's length prefix
        // and the argument bytes themselves.
        let bytes = self.buf.as_bytes();
        let l_prev = ((bytes[0] as u32) << 24)
            | ((bytes[1] as u32) << 16)
            | ((bytes[2] as u32) << 8)
            | (bytes[3] as u32);
        let skip = 4usize + l_prev as usize;
        self.buf.drain(..skip);

        // Read the current argument's length prefix.
        let bytes = self.buf.as_bytes();
        let l_curr = ((bytes[0] as u32) << 24)
            | ((bytes[1] as u32) << 16)
            | ((bytes[2] as u32) << 8)
            | (bytes[3] as u32);
        let len = l_curr as usize;

        // Return a slice into `self.buf` covering the argument bytes.
        // `self.buf` retains the length prefix at the front so that the
        // next call can correctly compute how far to advance.
        &self.buf[4..4 + len]
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
    let argc = argv.len() as u8;
    // Pre-compute total length: 1 byte header + 4 bytes per arg + arg data.
    let mut total = 1usize;
    for arg in argv {
        total += 4 + arg.as_bytes().len();
    }
    let mut buf: Vec<u8> = Vec::with_capacity(total);

    // Header byte: (version << 4) | (argc & 0x0f).
    buf.push(((AMP_VERSION as u8) << 4) | (argc & 0x0f));

    // Each argument: 4-byte big-endian length prefix followed by data.
    for arg in argv {
        let arg_bytes = arg.as_bytes();
        let len = arg_bytes.len() as u32;
        buf.push(((len >> 24) & 0xff) as u8);
        buf.push(((len >> 16) & 0xff) as u8);
        buf.push(((len >> 8) & 0xff) as u8);
        buf.push((len & 0xff) as u8);
        buf.extend_from_slice(arg_bytes);
    }

    String::from_utf8(buf).expect("AMP buffer must be valid UTF-8")
}
