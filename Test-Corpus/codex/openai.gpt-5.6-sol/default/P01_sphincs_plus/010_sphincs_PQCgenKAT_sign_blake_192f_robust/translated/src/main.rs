use sphincs_plus_translation::{
    CRYPTO_BYTES, CRYPTO_PUBLICKEYBYTES, CRYPTO_SECRETKEYBYTES, keypair, sign, verify,
};
use sphincs_plus_translation::{initialize_deterministic_rng, random_bytes};
use sphincs_plus_translation::transcript::KatTranscript;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

fn main() {
    let mut entropy_input = [0u8; 48];
    for (i, byte) in entropy_input.iter_mut().enumerate() {
        *byte = i as u8;
    }
    initialize_deterministic_rng(&entropy_input);

    let mut transcript = KatTranscript::new();
    transcript.absorb_label("CRYPTO_ALGNAME");
    transcript.absorb_bytes(b"SPHINCS+");
    transcript.absorb_label("SKBYTES");
    transcript.absorb_u64(CRYPTO_SECRETKEYBYTES as u64);
    transcript.absorb_label("PKBYTES");
    transcript.absorb_u64(CRYPTO_PUBLICKEYBYTES as u64);
    transcript.absorb_label("SIGBYTES");
    transcript.absorb_u64(CRYPTO_BYTES as u64);

    for count in 0..LOOP_COUNT {
        let mut seed = [0u8; 48];
        random_bytes(&mut seed);
        transcript.absorb_label("count");
        transcript.absorb_u64(count as u64);
        transcript.absorb_label("seed");
        transcript.absorb_bytes(&seed);

        let mlen = BASE_MLEN * (count + 1);
        transcript.absorb_label("mlen");
        transcript.absorb_u64(mlen as u64);

        let mut message = vec![0u8; mlen];
        random_bytes(&mut message);
        transcript.absorb_label("msg");
        transcript.absorb_bytes(&message);

        let keys = keypair();
        transcript.absorb_label("pk");
        transcript.absorb_bytes(&keys.public);
        transcript.absorb_label("sk");
        transcript.absorb_bytes(&keys.secret);

        let signature = sign(&message, &keys);
        let mut signed_message = Vec::with_capacity(CRYPTO_BYTES + mlen);
        signed_message.extend_from_slice(&signature);
        signed_message.extend_from_slice(&message);
        transcript.absorb_label("smlen");
        transcript.absorb_u64(signed_message.len() as u64);
        transcript.absorb_label("sm");
        transcript.absorb_bytes(&signed_message);

        if verify(&signature, &message, &keys).is_err() {
            eprintln!("crypto_sign_open=-1");
            std::process::exit(-2);
        }
    }

    let digest = transcript.finalize();
    print!("KAT transcript digest = ");
    for byte in digest {
        print!("{byte:02X}");
    }
    println!();
}
