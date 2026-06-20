use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use blake2b_simd::{Params, State};

pub const BLAKE2B_BLOCKBYTES: usize = 128;
pub const BLAKE2B_OUTBYTES: usize = 64;
pub const BLAKE2B_KEYBYTES: usize = 64;
pub const BLAKE2B_SALTBYTES: usize = 16;
pub const BLAKE2B_PERSONALBYTES: usize = 16;

const DEFAULT_PERSONAL: &[u8; BLAKE2B_PERSONALBYTES] = b"ckb-default-hash";

#[derive(Debug, Clone)]
pub struct Blake2bState {
    pub h: [u64; 8],
    pub t: [u64; 2],
    pub f: [u64; 2],
    pub buf: [u8; BLAKE2B_BLOCKBYTES],
    pub buflen: usize,
    pub outlen: usize,
    pub last_node: u8,
}

#[repr(packed)]
#[derive(Debug, Clone)]
pub struct Blake2bParam {
    pub digest_length: u8,
    pub key_length: u8,
    pub fanout: u8,
    pub depth: u8,
    pub leaf_length: u32,
    pub node_offset: u32,
    pub xof_length: u32,
    pub node_depth: u8,
    pub inner_length: u8,
    pub reserved: [u8; 14],
    pub salt: [u8; BLAKE2B_SALTBYTES],
    pub personal: [u8; BLAKE2B_PERSONALBYTES],
}

impl Default for Blake2bState {
    fn default() -> Self {
        Self {
            h: [0; 8],
            t: [0; 2],
            f: [0; 2],
            buf: [0; BLAKE2B_BLOCKBYTES],
            buflen: 0,
            outlen: 0,
            last_node: 0,
        }
    }
}

#[derive(Clone)]
struct StoredState {
    state: State,
    outlen: usize,
    finalized: bool,
}

fn state_map() -> &'static Mutex<HashMap<usize, StoredState>> {
    static STATE_MAP: OnceLock<Mutex<HashMap<usize, StoredState>>> = OnceLock::new();
    STATE_MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn state_key(state: &Blake2bState) -> usize {
    state as *const Blake2bState as usize
}

fn params_from_blake2b_param(param: &Blake2bParam) -> Result<Params, i32> {
    if param.digest_length == 0 || usize::from(param.digest_length) > BLAKE2B_OUTBYTES {
        return Err(-1);
    }
    if usize::from(param.key_length) > BLAKE2B_KEYBYTES {
        return Err(-1);
    }

    let mut params = Params::new();
    params.hash_length(usize::from(param.digest_length));
    params.fanout(param.fanout);
    params.max_depth(param.depth);
    params.max_leaf_length(param.leaf_length);
    params.node_offset(u64::from(param.node_offset));
    params.node_depth(param.node_depth);
    params.inner_hash_length(usize::from(param.inner_length));
    params.salt(&param.salt);
    params.personal(&param.personal);
    Ok(params)
}

fn install_state(dest: &mut Blake2bState, state: State, outlen: usize) {
    *dest = Blake2bState::default();
    dest.outlen = outlen;
    state_map().lock().unwrap().insert(
        state_key(dest),
        StoredState {
            state,
            outlen,
            finalized: false,
        },
    );
}

pub fn blake2b_init(state: &mut Blake2bState, outlen: usize) -> i32 {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES {
        return -1;
    }

    let mut params = Params::new();
    params.hash_length(outlen);
    params.personal(DEFAULT_PERSONAL);
    install_state(state, params.to_state(), outlen);
    0
}

pub fn blake2b_init_key(state: &mut Blake2bState, outlen: usize, key: &[u8]) -> i32 {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || key.is_empty() || key.len() > BLAKE2B_KEYBYTES {
        return -1;
    }

    let mut params = Params::new();
    params.hash_length(outlen);
    params.key(key);
    install_state(state, params.to_state(), outlen);
    0
}

pub fn blake2b_init_param(state: &mut Blake2bState, param: &Blake2bParam) -> i32 {
    let params = match params_from_blake2b_param(param) {
        Ok(params) => params,
        Err(err) => return err,
    };
    install_state(state, params.to_state(), usize::from(param.digest_length));
    0
}

pub fn blake2b_update(state: &mut Blake2bState, input: &[u8]) -> i32 {
    let key = state_key(state);
    let mut guard = state_map().lock().unwrap();
    let Some(stored) = guard.get_mut(&key) else {
        return -1;
    };

    stored.state.update(input);
    state.buflen = (state.buflen + input.len()) % BLAKE2B_BLOCKBYTES;
    0
}

pub fn blake2b_final(state: &mut Blake2bState, out: &mut [u8]) -> i32 {
    let key = state_key(state);
    let mut guard = state_map().lock().unwrap();
    let Some(stored) = guard.get_mut(&key) else {
        return -1;
    };
    if stored.finalized || out.len() < stored.outlen {
        return -1;
    }

    let hash = stored.state.finalize();
    out[..stored.outlen].copy_from_slice(&hash.as_bytes()[..stored.outlen]);
    stored.finalized = true;
    state.f[0] = u64::MAX;
    0
}

pub fn blake2b(out: &mut [u8], input: &[u8], key: Option<&[u8]>) -> i32 {
    if out.is_empty() || out.len() > BLAKE2B_OUTBYTES {
        return -1;
    }
    if key.is_some_and(|key_bytes| key_bytes.len() > BLAKE2B_KEYBYTES) {
        return -1;
    }

    let mut state = Blake2bState::default();
    let init_result = match key {
        Some(key_bytes) if !key_bytes.is_empty() => blake2b_init_key(&mut state, out.len(), key_bytes),
        _ => blake2b_init(&mut state, out.len()),
    };
    if init_result != 0 {
        return init_result;
    }
    if blake2b_update(&mut state, input) != 0 {
        return -1;
    }
    blake2b_final(&mut state, out)
}

pub fn blake2(out: &mut [u8], input: &[u8], key: Option<&[u8]>) -> i32 {
    blake2b(out, input, key)
}
