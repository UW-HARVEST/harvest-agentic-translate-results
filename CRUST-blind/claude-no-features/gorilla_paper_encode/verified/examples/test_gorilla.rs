use gorilla_paper_encode::gorilla::*;

fn main() {
    let mut encoder = FloatEncoder {
        w: BitWriter { cache: [0; 1024], pos: 0, byte: 0, bit_count: 0 },
        val: 0,
        leading: 0,
        trailing: 0,
        first: false,
        finished: false,
    };
    encoder.float_encoder_init();
    
    let arr: [f64; 8] = [2300.0, 2400.0, 2500.0, 2600.0, 2700.0, 2800.0, 2900.0, 3000.0];
    for &v in &arr {
        encoder.float_encode_write(v);
    }
    
    let mut buffer = [0u8; 1024];
    let mut length: u32 = 0;
    encoder.float_encode_flush(&mut buffer, &mut length);
    
    println!("Encoded length: {}", length);
    print!("Bytes: ");
    for i in 0..length as usize {
        print!("{:02x} ", buffer[i]);
    }
    println!();
    
    let mut decoder = FloatDecoder {
        val: 0,
        leading: 0,
        trailing: 0,
        br: BitReader { data: &[], len: 0, v: 0, n: 0 },
        b: [0; 1024],
        first: false,
        finished: false,
        err: 0,
    };
    
    let mut de_arr = [0.0f64; 64];
    let mut de_len: u32 = 0;
    decoder.float_decode_block(&buffer[..length as usize], &mut de_arr, &mut de_len);
    
    println!("Decoded {} values:", de_len);
    for i in 0..de_len as usize {
        println!("  {}", de_arr[i]);
    }
    
    assert_eq!(de_len, 8);
    for i in 0..8 {
        assert_eq!(de_arr[i], arr[i]);
    }
    println!("All tests passed!");
}
