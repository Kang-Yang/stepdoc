use std::sync::Mutex;

use crate::models::KeyBuffer;

pub fn clear_key_buffer(key_buffer: &Mutex<KeyBuffer>) {
    let Ok(mut pending) = key_buffer.lock() else {
        return;
    };
    pending.chars.clear();
    pending.last_event = 0;
}
