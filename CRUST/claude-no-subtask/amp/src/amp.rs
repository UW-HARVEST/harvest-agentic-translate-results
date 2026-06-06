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
        self.version = ((header >> 4) & 0xf) as i16;
        self.argc = (header & 0xf) as i16;
        // Store remainder so subsequent decode_arg calls can consume args.
        self.buf = String::from_utf8(bytes[1..].to_vec())
            .expect("buffer must be valid utf-8");
    }
    /// Decodes and returns the next argument from the message.
    ///
    /// # Returns
    ///
    /// A string slice representing the next decoded argument.
    pub fn decode_arg(&mut self) -> &str {
        let bytes = self.buf.as_bytes();
        let len = ((bytes[0] as u32) << 24)
            | ((bytes[1] as u32) << 16)
            | ((bytes[2] as u32) << 8)
            | (bytes[3] as u32);
        let len = len as usize;
        let chunk_size = 4 + len;

        // Rotate: move the [length prefix + arg] chunk to the END of self.buf.
        // Before: self.buf = [len_n][arg_n][len_(n+1)][arg_(n+1)]...
        // After:  self.buf = [len_(n+1)][arg_(n+1)]...[len_n][arg_n]
        // The returned &str references the trailing arg bytes, which remain
        // valid until the next mutable borrow of self.
        let prefix: String = self.buf[..chunk_size].to_string();
        let suffix: String = self.buf[chunk_size..].to_string();
        self.buf = suffix;
        self.buf.push_str(&prefix);

        let total = self.buf.len();
        &self.buf[total - len..total]
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

    // Version/argc header byte: top 4 bits = version, bottom 4 bits = argc.
    let header = (((AMP_VERSION as u8) & 0xf) << 4) | ((argc as u8) & 0xf);
    bytes.push(header);

    for arg in argv {
        let arg_bytes = arg.as_bytes();
        let len = arg_bytes.len() as u32;
        bytes.push(((len >> 24) & 0xff) as u8);
        bytes.push(((len >> 16) & 0xff) as u8);
        bytes.push(((len >> 8) & 0xff) as u8);
        bytes.push((len & 0xff) as u8);
        bytes.extend_from_slice(arg_bytes);
    }

    String::from_utf8(bytes).expect("encoded buffer must be valid utf-8")
}
