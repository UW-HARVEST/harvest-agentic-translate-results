mod tables;
use tables::TFLAC_CRC16_TABLES;

// Type aliases mirroring the C header
#[allow(non_camel_case_types)]
pub type tflac_u8 = u8;
#[allow(non_camel_case_types)]
pub type tflac_u16 = u16;
#[allow(non_camel_case_types)]
pub type tflac_u32 = u32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crc16(
    d: *const tflac_u8,
    len: tflac_u32,
    crc16: tflac_u16,
) -> tflac_u16 {
    let mut d = d;
    let mut len = len;
    let mut crc: tflac_u16 = crc16;

    while len >= 8 {
        // crc16 ^= d[0] << 8 | d[1];
        // In C: (d[0] << 8 | d[1]) is computed in int (i32 promotion). The result fits in u16.
        let b0 = unsafe { *d.add(0) } as tflac_u16;
        let b1 = unsafe { *d.add(1) } as tflac_u16;
        crc ^= (b0 << 8) | b1;

        let b2 = unsafe { *d.add(2) } as usize;
        let b3 = unsafe { *d.add(3) } as usize;
        let b4 = unsafe { *d.add(4) } as usize;
        let b5 = unsafe { *d.add(5) } as usize;
        let b6 = unsafe { *d.add(6) } as usize;
        let b7 = unsafe { *d.add(7) } as usize;

        crc = TFLAC_CRC16_TABLES[7][(crc >> 8) as usize]
            ^ TFLAC_CRC16_TABLES[6][(crc & 0xFF) as usize]
            ^ TFLAC_CRC16_TABLES[5][b2]
            ^ TFLAC_CRC16_TABLES[4][b3]
            ^ TFLAC_CRC16_TABLES[3][b4]
            ^ TFLAC_CRC16_TABLES[2][b5]
            ^ TFLAC_CRC16_TABLES[1][b6]
            ^ TFLAC_CRC16_TABLES[0][b7];

        d = unsafe { d.add(8) };
        len -= 8;
    }

    // while (len--) emulates post-decrement: loops while len != 0, decrements after test
    while len != 0 {
        len = len.wrapping_sub(1);
        let byte = unsafe { *d } as tflac_u16;
        d = unsafe { d.add(1) };
        // crc16 = (crc16 << 8) ^ tflac_crc16_tables[0][(crc16 >> 8) ^ *d++];
        let idx = ((crc >> 8) ^ byte) as usize & 0xFF;
        crc = (crc << 8) ^ TFLAC_CRC16_TABLES[0][idx];
    }

    crc
}
