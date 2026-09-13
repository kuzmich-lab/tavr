#![no_std]
fn get_ascii_str<'a>(buffer: &'a [u8]) -> Result<&'a str, ()> {
    for byte in buffer.into_iter() {
        if byte >= &128 {
            return Err(());
        }
    }
    Ok(unsafe {
        // This is safe because we verified above that it's a valid ASCII
        // string, and all ASCII strings are also UTF8 strings
        core::str::from_utf8_unchecked(buffer)
    })
}
