pub mod macros;
pub mod test;

/// Format a byte slice into a hex display string
pub fn hex_display_slice(slice: &[u8]) -> String {
    let mut result = String::new();

    for (chunk_index, chunk) in slice.chunks(16).enumerate() {
        draw_range(&mut result, chunk_index * 16, chunk);
    }

    // Remove trailing newline
    if result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Format a byte vector into a hex display string
pub fn hex_display_vec(vec: Vec<u8>) -> String {
    hex_display_slice(&vec)
}

/// Draw one line of hex display
fn draw_range(result: &mut String, offset: usize, chunk: &[u8]) {
    write_offset(result, offset);
    write_data(result, chunk);
    write_ascii(result, chunk);
    result.push('\n');
}

/// Write the offset part
fn write_offset(result: &mut String, offset: usize) {
    result.push_str(&format!("{:08x}  ", offset));
}

/// Write the hex data part
fn write_data(result: &mut String, chunk: &[u8]) {
    for (i, &byte) in chunk.iter().enumerate() {
        result.push_str(&format!("{:02x}", byte));

        if i < 15 {
            result.push(' ');
        }

        if i == 7 {
            result.push(' ');
        }
    }

    if chunk.len() < 16 {
        pad_data(result, chunk.len());
    }
}

/// Pad missing hex data
fn pad_data(result: &mut String, actual_len: usize) {
    let missing = 16 - actual_len;

    for i in 0..missing {
        result.push_str("   ");

        if actual_len + i == 7 {
            result.push(' ');
        }
    }

    if result.ends_with(' ') {
        result.pop();
    }
}

/// Write the ASCII representation part
fn write_ascii(result: &mut String, chunk: &[u8]) {
    result.push_str("  |");

    for &byte in chunk {
        if byte >= 32 && byte <= 126 {
            result.push(byte as char);
        } else {
            result.push('.');
        }
    }

    pad_ascii(result, chunk.len());

    result.push('|');
}

/// Pad the ASCII part to a fixed width
fn pad_ascii(result: &mut String, actual_len: usize) {
    for _ in actual_len..16 {
        result.push(' ');
    }
}
