use crate::psbt::*;

fn usage() -> i32 {
    println!("usage: psbt <psbt>");
    1
}

#[allow(dead_code)]
fn main() -> i32 {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return usage();
    }

    let mut psbt = Psbt::new(4096);
    let mut buffer = vec![0u8; 4096];

    let res = psbt_init(&mut psbt, &mut buffer, 4096);
    if res != PsbtResult::Ok {
        println!("error: {}", psbt_geterr());
        return 1;
    }

    let psbt_hex = &args[1];
    let psbt_hex_len = psbt_hex.len();
    let mut psbt_len = 0usize;

    let res = psbt_decode(psbt_hex, psbt_hex_len, &mut buffer, 4096, &mut psbt_len);
    if res != PsbtResult::Ok {
        println!("error: {}", psbt_geterr());
        return 1;
    }

    let buf_copy = buffer.clone();
    let res = psbt_read(&buf_copy, psbt_len, &mut psbt, None, &mut ());
    if res != PsbtResult::Ok {
        println!("error: {}", psbt_geterr());
        return 1;
    }

    let mut out_len = 0usize;
    let res = psbt_encode(
        &psbt,
        PsbtEncoding::Base62,
        &mut buffer,
        4096,
        &mut out_len,
    );
    if res != PsbtResult::Ok {
        println!("error: {}", psbt_geterr());
        return 1;
    }

    0
}
