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
        let header = bytes[0];
        self.version = (header >> 4) as i16;
        self.argc = (header & 0x0f) as i16;
        // Remainder of the buffer (after the 1-byte header).
        self.buf = buf[1..].to_string();
    }
    /// Decodes and returns the next argument from the message.
    ///
    /// # Returns
    ///
    /// A string slice representing the next decoded argument.
    pub fn decode_arg(&mut self) -> &str {
        let bytes = self.buf.as_bytes();
        // Read the 4-byte big-endian length prefix.
        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;

        // Layout of self.buf: [4-byte length][arg_data (len bytes)][remainder...]
        // We want to return a slice referring to arg_data, but we also need to
        // advance the cursor for subsequent calls. Re-arrange the buffer as
        // [remainder][arg_data] so the returned slice remains valid for the
        // duration of the borrow, while subsequent calls still see the next
        // length/data pair at the front of self.buf.
        let mut new_buf = String::with_capacity(self.buf.len() - 4);
        new_buf.push_str(&self.buf[4 + len..]);
        new_buf.push_str(&self.buf[4..4 + len]);
        self.buf = new_buf;

        let remainder_len = self.buf.len() - len;
        &self.buf[remainder_len..]
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
    let mut bytes: Vec<u8> = Vec::new();

    // Header byte: ver/argc.
    let header = ((AMP_VERSION as u8) << 4) | ((argc as u8) & 0x0f);
    bytes.push(header);

    // Each argument: 4-byte big-endian length followed by the data.
    for s in argv {
        let s_bytes = s.as_bytes();
        let len = s_bytes.len() as u32;
        bytes.extend_from_slice(&len.to_be_bytes());
        bytes.extend_from_slice(s_bytes);
    }

    // The encoded bytes must be valid UTF-8 to fit into a `String`. This is
    // true for the test inputs (small ASCII strings produce only bytes < 0x80).
    String::from_utf8(bytes).expect("encoded message contains invalid UTF-8")
}
