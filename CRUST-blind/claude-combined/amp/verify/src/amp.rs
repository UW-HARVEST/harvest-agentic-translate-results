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
        let b0 = bytes[0];
        self.version = ((b0 >> 4) & 0x0f) as i16;
        self.argc = (b0 & 0x0f) as i16;
        // Remaining bytes (the cursor starts after the header byte).
        let rest: Vec<u8> = bytes[1..].to_vec();
        self.buf = String::from_utf8(rest).expect("amp: buffer is not valid UTF-8");
    }
    /// Decodes and returns the next argument from the message.
    ///
    /// # Returns
    ///
    /// A string slice representing the next decoded argument.
    pub fn decode_arg(&mut self) -> &str {
        // Take ownership of the current buf so we can rearrange it.
        let buf = std::mem::take(&mut self.buf);
        let bytes = buf.as_bytes();
        // Read u32 big-endian length.
        let len = ((bytes[0] as u32) << 24)
            | ((bytes[1] as u32) << 16)
            | ((bytes[2] as u32) << 8)
            | (bytes[3] as u32);
        let len = len as usize;

        let arg_bytes = bytes[4..4 + len].to_vec();
        let rest_bytes = bytes[4 + len..].to_vec();

        // Layout new buf as: <rest_of_message><current_arg>
        // This keeps the cursor at the start of the remaining args, while
        // preserving the just-decoded argument at the tail so we can return
        // a &str borrowed from self.buf.
        let mut new_bytes = Vec::with_capacity(rest_bytes.len() + arg_bytes.len());
        new_bytes.extend_from_slice(&rest_bytes);
        new_bytes.extend_from_slice(&arg_bytes);
        self.buf = String::from_utf8(new_bytes).expect("amp: arg is not valid UTF-8");

        let total = self.buf.len();
        &self.buf[total - len..]
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
    // Compute total length: 1 (header) + sum(4 + len(arg)).
    let mut total: usize = 1;
    let mut lens: Vec<usize> = Vec::with_capacity(argv.len());
    for arg in argv {
        total += 4;
        let l = arg.len();
        lens.push(l);
        total += l;
    }

    let mut buf: Vec<u8> = Vec::with_capacity(total);
    // ver/argc byte: AMP_VERSION << 4 | argc
    let header = ((AMP_VERSION as u8) << 4) | (argc & 0x0f);
    buf.push(header);

    for (i, arg) in argv.iter().enumerate() {
        let len = lens[i] as u32;
        buf.push(((len >> 24) & 0xff) as u8);
        buf.push(((len >> 16) & 0xff) as u8);
        buf.push(((len >> 8) & 0xff) as u8);
        buf.push((len & 0xff) as u8);
        buf.extend_from_slice(arg.as_bytes());
    }

    String::from_utf8(buf).expect("amp: encoded buffer is not valid UTF-8")
}
