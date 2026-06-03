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
        // Remaining buffer (after the 1-byte header) holds the encoded args.
        let rest = bytes[1..].to_vec();
        self.buf = String::from_utf8(rest).unwrap_or_default();
    }
    /// Decodes and returns the next argument from the message.
    ///
    /// # Returns
    ///
    /// A string slice representing the next decoded argument.
    pub fn decode_arg(&mut self) -> &str {
        // Parse the 4-byte big-endian length header at the front of buf.
        let bytes = self.buf.as_bytes();
        let len = ((bytes[0] as u32) << 24)
            | ((bytes[1] as u32) << 16)
            | ((bytes[2] as u32) << 8)
            | (bytes[3] as u32);
        let len = len as usize;

        let arg_start = 4;
        let arg_end = arg_start + len;

        // Split buffer into the just-parsed arg bytes and the remaining
        // (still-encoded) data following it.
        let arg_bytes = self.buf.as_bytes()[arg_start..arg_end].to_vec();
        let remaining_bytes = self.buf.as_bytes()[arg_end..].to_vec();

        // Rebuild self.buf as: <remaining_encoded_bytes><arg_bytes>
        // - The remaining encoded bytes are at the front so the next call to
        //   decode_arg can parse them as usual.
        // - The arg bytes are appended at the end so we can return a slice
        //   borrowed from self.buf that lives long enough.
        let mut new_buf = Vec::with_capacity(remaining_bytes.len() + arg_bytes.len());
        new_buf.extend_from_slice(&remaining_bytes);
        let arg_pos = new_buf.len();
        new_buf.extend_from_slice(&arg_bytes);

        self.buf = String::from_utf8(new_buf).unwrap_or_default();

        &self.buf[arg_pos..arg_pos + len]
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

    // Header byte: top nibble is the protocol version, bottom nibble argc.
    let header: u8 = ((AMP_VERSION as u8) << 4) | ((argc as u8) & 0x0f);
    buf.push(header);

    // For each arg: 4-byte big-endian length, followed by the arg bytes.
    for arg in argv {
        let len = arg.len() as u32;
        buf.push(((len >> 24) & 0xff) as u8);
        buf.push(((len >> 16) & 0xff) as u8);
        buf.push(((len >> 8) & 0xff) as u8);
        buf.push((len & 0xff) as u8);
        buf.extend_from_slice(arg.as_bytes());
    }

    String::from_utf8(buf).unwrap_or_default()
}
