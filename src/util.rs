use std::sync::atomic::{AtomicUsize, Ordering};

static GLOBAL_ID: AtomicUsize = AtomicUsize::new(0);

pub fn generate_id() -> usize {
    GLOBAL_ID.fetch_add(1, Ordering::SeqCst)
}

pub fn extract_bits(value: &[u8], bytes: usize) -> Vec<u8> {
    let mut bits = vec![];

    for j in 0..bytes {
        for i in 0..8 {
            let mask = 1 << i;
            // Check if the bit is set and store 1 or 0 accordingly
            let bit = if (value[j] & mask) != 0 { 1 } else { 0 };

            bits.push(bit);
        }
    }

    bits
}
