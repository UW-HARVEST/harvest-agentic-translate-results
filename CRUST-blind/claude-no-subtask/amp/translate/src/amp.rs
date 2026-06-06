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
        if bytes.is_empty() {
            self.version = 0;
            self.argc = 0;
            self.buf = String::new();
            return;
        }
        let header = bytes[0];
        self.version = ((header >> 4) & 0x0f) as i16;
        self.argc = (header & 0x0f) as i16;
        // Store the rest of the buffer (after the header byte). The bytes may
        // contain arbitrary binary data (e.g., 4-byte length prefixes), which
        // is not necessarily valid UTF-8, so we construct the String unchecked.
        let rest: Vec<u8> = bytes[1..].to_vec();
        // SAFETY: We treat `String` as a byte container for this binary
        // protocol. We never use UTF-8-dependent String/str operations
        // (only `as_bytes` and indexed slicing within byte boundaries).
        self.buf = unsafe { String::from_utf8_unchecked(rest) };
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
        let data_start = 4usize;
        let data_end = data_start + len;

        // Reconstruct the buffer so that:
        //   - the remaining encoded args (after this arg's data) come first,
        //   - this arg's data is appended to the end.
        // This lets us advance the read cursor while keeping the just-decoded
        // data alive in `self.buf`, so we can return a slice into it.
        let mut new_buf: Vec<u8> = Vec::with_capacity(self.buf.len().saturating_sub(4));
        new_buf.extend_from_slice(&bytes[data_end..]);
        new_buf.extend_from_slice(&bytes[data_start..data_end]);

        // SAFETY: see comment in `decode`. We use `String` as a byte container.
        self.buf = unsafe { String::from_utf8_unchecked(new_buf) };

        let total_len = self.buf.len();
        let slice_start = total_len - len;
        // SAFETY: The argument bytes were originally provided as `&str` via
        // `amp_encode`, so the slice is valid UTF-8 in the typical use case.
        // We use the unchecked variant to remain consistent with our
        // binary-buffer representation.
        unsafe { std::str::from_utf8_unchecked(&self.buf.as_bytes()[slice_start..]) }
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

    // Compute total length: 1 byte header + sum(4 + arg.len()) for each arg.
    let total_len: usize = 1 + argv.iter().map(|s| 4 + s.len()).sum::<usize>();
    let mut buf: Vec<u8> = Vec::with_capacity(total_len);

    // Header byte: AMP_VERSION (high nibble) | argc (low nibble).
    let header: u8 = ((AMP_VERSION as u8) << 4) | ((argc as u8) & 0x0f);
    buf.push(header);

    // Encode each argument: 4-byte big-endian length, then bytes.
    for arg in argv {
        let len = arg.len() as u32;
        buf.extend_from_slice(&len.to_be_bytes());
        buf.extend_from_slice(arg.as_bytes());
    }

    // SAFETY: The encoded buffer is binary (lengths are arbitrary bytes), so
    // it's not necessarily valid UTF-8. We treat `String` here as a byte
    // container for the binary protocol; downstream consumers of this buffer
    // (i.e. `Amp::decode`) only use `as_bytes()`.
    unsafe { String::from_utf8_unchecked(buf) }
}
