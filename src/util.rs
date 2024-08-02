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

pub fn unpack_shorts(buf: &[u8], v1: &mut u16, v2: &mut u16) {
    *v1 = (((buf[1] as u16) << 8) & 0x0f00) | buf[0] as u16;
    *v2 = ((buf[2] as u16) << 4) | ((buf[1] as u16) >> 4);
}

pub fn is_dead_zone(x: u16, y: u16, x_center: u16, y_center: u16, dead_zone: u16) -> bool {
    let dx = x - x_center;
    let dy = y - y_center;
    if dx * dx + dy * dy < dead_zone * dead_zone {
        return true;
    }
    false
}

pub fn clamp_axis(value: u16, min: u16, max: u16) -> f32 {
    // Clamp the value between min and max
    if value <= min {
        return -1.0;
    } else if value >= max {
        return 1.0;
    } else {
        return 2.0 * (value - min) as f32 / (max - min) as f32 - 1.0;
    }
}
