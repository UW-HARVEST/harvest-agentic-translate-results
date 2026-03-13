static TFLAC_CRC16_TABLES: [[u16; 256]; 8] = [
    include!("table0.rs"),
    include!("table1.rs"),
    include!("table2.rs"),
    include!("table3.rs"),
    include!("table4.rs"),
    include!("table5.rs"),
    include!("table6.rs"),
    include!("table7.rs"),
];

#[unsafe(no_mangle)]
pub extern "C" fn crc16(d: *const u8, len: u32, mut crc: u16) -> u16 {
    let data = unsafe { core::slice::from_raw_parts(d, len as usize) };
    let mut i = 0usize;
    let mut remaining = len as usize;

    while remaining >= 8 {
        crc ^= (data[i] as u16) << 8 | data[i + 1] as u16;
        crc = TFLAC_CRC16_TABLES[7][(crc >> 8) as usize]
            ^ TFLAC_CRC16_TABLES[6][(crc & 0xFF) as usize]
            ^ TFLAC_CRC16_TABLES[5][data[i + 2] as usize]
            ^ TFLAC_CRC16_TABLES[4][data[i + 3] as usize]
            ^ TFLAC_CRC16_TABLES[3][data[i + 4] as usize]
            ^ TFLAC_CRC16_TABLES[2][data[i + 5] as usize]
            ^ TFLAC_CRC16_TABLES[1][data[i + 6] as usize]
            ^ TFLAC_CRC16_TABLES[0][data[i + 7] as usize];
        i += 8;
        remaining -= 8;
    }
    while remaining > 0 {
        crc = (crc << 8) ^ TFLAC_CRC16_TABLES[0][((crc >> 8) ^ data[i] as u16) as usize];
        i += 1;
        remaining -= 1;
    }
    crc
}
