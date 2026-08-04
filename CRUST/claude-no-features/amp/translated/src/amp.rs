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
        self.version = ((header >> 4) & 0x0f) as i16;
        self.argc = (header & 0x0f) as i16;
        // Internal layout: [4-byte big-endian length of "current arg"] +
        //                  [current arg bytes] +
        //                  [remaining encoded args still length-prefixed].
        // After `decode` there is no current arg yet, so we prepend a zero
        // length sentinel.
        let mut new_buf = Vec::with_capacity(4 + bytes.len() - 1);
        new_buf.extend_from_slice(&[0u8, 0, 0, 0]);
        new_buf.extend_from_slice(&bytes[1..]);
        self.buf = String::from_utf8(new_buf)
            .expect("amp buffer is not valid UTF-8");
    }
    /// Decodes and returns the next argument from the message.
    ///
    /// # Returns
    ///
    /// A string slice representing the next decoded argument.
    pub fn decode_arg(&mut self) -> &str {
        let bytes = self.buf.as_bytes();
        // Skip past the previously-returned argument (whose length is stored
        // at the front of self.buf as a 4-byte big-endian integer).
        let prev_len = ((bytes[0] as u32) << 24)
            | ((bytes[1] as u32) << 16)
            | ((bytes[2] as u32) << 8)
            | (bytes[3] as u32);
        let after_prev = &bytes[4 + prev_len as usize..];

        // Read the length prefix of the next argument.
        let next_len = ((after_prev[0] as u32) << 24)
            | ((after_prev[1] as u32) << 16)
            | ((after_prev[2] as u32) << 8)
            | (after_prev[3] as u32);
        let next_len_usize = next_len as usize;
        let next_arg = &after_prev[4..4 + next_len_usize];
        let rest = &after_prev[4 + next_len_usize..];

        // Rebuild self.buf so the just-returned argument occupies bytes 4..4+next_len
        // and the remaining encoded arguments follow.
        let mut new_buf = Vec::with_capacity(4 + next_len_usize + rest.len());
        new_buf.push(((next_len >> 24) & 0xff) as u8);
        new_buf.push(((next_len >> 16) & 0xff) as u8);
        new_buf.push(((next_len >> 8) & 0xff) as u8);
        new_buf.push((next_len & 0xff) as u8);
        new_buf.extend_from_slice(next_arg);
        new_buf.extend_from_slice(rest);
        self.buf = String::from_utf8(new_buf)
            .expect("amp buffer is not valid UTF-8");

        &self.buf[4..4 + next_len_usize]
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
    let mut buf: Vec<u8> = Vec::new();

    // Version (high nibble) and argc (low nibble) header byte.
    let header = ((AMP_VERSION as u8) << 4) | (argc as u8 & 0x0f);
    buf.push(header);

    for arg in argv {
        let arg_bytes = arg.as_bytes();
        let len = arg_bytes.len() as u32;
        buf.push(((len >> 24) & 0xff) as u8);
        buf.push(((len >> 16) & 0xff) as u8);
        buf.push(((len >> 8) & 0xff) as u8);
        buf.push((len & 0xff) as u8);
        buf.extend_from_slice(arg_bytes);
    }

    String::from_utf8(buf).expect("amp encoded buffer is not valid UTF-8")
}
