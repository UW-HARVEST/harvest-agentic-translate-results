use sphincs_plus::api::{CRYPTO_ALGNAME, CRYPTO_BYTES, CRYPTO_PUBLICKEYBYTES, CRYPTO_SECRETKEYBYTES};
use sphincs_plus::sign::{crypto_sign, crypto_sign_keypair, crypto_sign_open};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

fn main() {
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
    println!("KAT transcript digest = {:02X?}", {
        let mut acc = sha2::Sha256::new();
        use sha2::Digest;
        acc.update(CRYPTO_ALGNAME.as_bytes());
        for i in 0..LOOP_COUNT {
            let mlen = BASE_MLEN * (i + 1);
            for (j, b) in msg[..mlen].iter_mut().enumerate() {
                *b = (i as u8).wrapping_add(j as u8);
            }
            let _ = crypto_sign_keypair(&mut pk, &mut sk);
            let mut smlen = 0u64;
            let _ = crypto_sign(&mut sm, &mut smlen, &msg[..mlen], &sk);
            let mut mlen1 = 0u64;
            let _ = crypto_sign_open(&mut m1, &mut mlen1, &sm[..smlen as usize], &pk);
            acc.update(&pk);
            acc.update(&sk);
            acc.update(&sm[..smlen as usize]);
        }
        let d = acc.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&d);
        arr
    }.iter().map(|b| format!("{:02X}", b)).collect::<String>());
}
