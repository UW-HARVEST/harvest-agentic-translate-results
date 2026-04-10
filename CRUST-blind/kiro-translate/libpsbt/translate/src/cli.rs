use crate::psbt::*;

fn main() -> i32 {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("usage: psbt <psbt>");
        return 1;
    }

    let mut psbt = Psbt::new(4096);
    let mut buffer = vec![0u8; 4096];
    let mut psbt_len = 0usize;
    let mut out_len = 0usize;

    let psbt_hex_str = &args[1];

    psbt_init(&mut psbt, &mut buffer, 4096);

    let res = psbt_decode(psbt_hex_str, psbt_hex_str.len(), &mut buffer, 4096, &mut psbt_len);
    if res != PsbtResult::Ok {
        println!("error ({:?}): {}. last_state = {}", res, psbt_geterr(), psbt_state_tostr(PsbtState::Init));
        return 1;
    }

    let buf_copy = buffer[..psbt_len].to_vec();
    let mut dummy: () = ();
    let res = psbt_read(&buf_copy, psbt_len, &mut psbt, None, &mut dummy);
    if res != PsbtResult::Ok {
        println!("error ({:?}): {}", res, psbt_geterr());
        return 1;
    }

    let res = psbt_encode(&psbt, PsbtEncoding::Base62, &mut buffer, 4096, &mut out_len);
    if res != PsbtResult::Ok {
        println!("error ({:?}): {}", res, psbt_geterr());
        return 1;
    }

    0
}
