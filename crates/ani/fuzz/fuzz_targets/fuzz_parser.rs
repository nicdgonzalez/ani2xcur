#![no_main]

use libfuzzer_sys::fuzz_target;

// (ignore -- i'm trying to get better at testing code)
fuzz_target!(|data: &[u8]| {
    _ = ani::de::Ani::from_bytes(data);
});
