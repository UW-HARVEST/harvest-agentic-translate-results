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
        self.version = (bytes[0] >> 4) as i16;
        self.argc = (bytes[0] & 0xf) as i16;
        self.buf = buf[1..].to_string();
    }
    /// Decodes and returns the next argument from the message.
    ///
    /// # Returns
    ///
    /// A string slice representing the next decoded argument.
    pub fn decode_arg(&mut self) -> &str {
        let bytes = self.buf.as_bytes();
        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let arg: String = self.buf[4..4 + len].to_string();
        self.buf.drain(..4 + len);
        Box::leak(arg.into_boxed_str())
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
    let mut total_len: usize = 1;
    let mut lens: Vec<usize> = Vec::with_capacity(argc);
    for arg in argv {
        total_len += 4;
        let l = arg.len();
        lens.push(l);
        total_len += l;
    }

    let mut buf: Vec<u8> = Vec::with_capacity(total_len);

    // ver/argc byte
    let header: u8 = ((AMP_VERSION as u8) << 4) | (argc as u8 & 0x0f);
    buf.push(header);

    for (i, arg) in argv.iter().enumerate() {
        let len = lens[i] as u32;
        buf.extend_from_slice(&len.to_be_bytes());
        buf.extend_from_slice(arg.as_bytes());
    }

    // SAFETY: bytes written are well-formed: header byte may be non-ASCII but
    // we use String to mirror the API. Use unsafe constructor to allow non-UTF-8
    // bytes to be stored.
    unsafe { String::from_utf8_unchecked(buf) }
}