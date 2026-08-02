use crate::PatchError;

pub(crate) const REPLACEMENT: u32 = 0xFFFD;

pub(crate) fn decode_go_runes(bytes: &[u8]) -> Vec<u32> {
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let (rune, width) = decode_go_rune(&bytes[i..]);
        result.push(rune);
        i += width;
    }
    result
}

pub(crate) fn decode_go_rune(bytes: &[u8]) -> (u32, usize) {
    if bytes.is_empty() {
        return (REPLACEMENT, 0);
    }
    let first = bytes[0];
    if first < 0x80 {
        return (u32::from(first), 1);
    }

    if (0xC2..=0xDF).contains(&first) && bytes.len() >= 2 && continuation(bytes[1]) {
        let rune = (u32::from(first & 0x1F) << 6) | u32::from(bytes[1] & 0x3F);
        return (rune, 2);
    }

    if (0xE0..=0xEF).contains(&first) && bytes.len() >= 3 {
        let second_ok = continuation(bytes[1])
            && !(first == 0xE0 && bytes[1] < 0xA0)
            && !(first == 0xED && bytes[1] >= 0xA0);
        if second_ok && continuation(bytes[2]) {
            let rune = (u32::from(first & 0x0F) << 12)
                | (u32::from(bytes[1] & 0x3F) << 6)
                | u32::from(bytes[2] & 0x3F);
            return (rune, 3);
        }
    }

    if (0xF0..=0xF4).contains(&first) && bytes.len() >= 4 {
        let second_ok = continuation(bytes[1])
            && !(first == 0xF0 && bytes[1] < 0x90)
            && !(first == 0xF4 && bytes[1] >= 0x90);
        if second_ok && continuation(bytes[2]) && continuation(bytes[3]) {
            let rune = (u32::from(first & 0x07) << 18)
                | (u32::from(bytes[1] & 0x3F) << 12)
                | (u32::from(bytes[2] & 0x3F) << 6)
                | u32::from(bytes[3] & 0x3F);
            return (rune, 4);
        }
    }

    (REPLACEMENT, 1)
}

fn continuation(byte: u8) -> bool {
    byte & 0xC0 == 0x80
}

pub(crate) fn encode_go_runes(runes: &[u32]) -> Vec<u8> {
    let mut output = Vec::with_capacity(runes.len());
    for &rune in runes {
        let ch = char::from_u32(rune).unwrap_or('\u{FFFD}');
        let mut buffer = [0_u8; 4];
        output.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
    }
    output
}

pub(crate) fn rune_count(bytes: &[u8]) -> usize {
    let mut count = 0;
    let mut index = 0;
    while index < bytes.len() {
        let (_, width) = decode_go_rune(&bytes[index..]);
        index += width.max(1);
        count += 1;
    }
    count
}

pub(crate) fn common_prefix_runes(first: &[u32], second: &[u32]) -> usize {
    first
        .iter()
        .zip(second)
        .take_while(|(left, right)| left == right)
        .count()
}

pub(crate) fn common_suffix_runes(first: &[u32], second: &[u32]) -> usize {
    first
        .iter()
        .rev()
        .zip(second.iter().rev())
        .take_while(|(left, right)| left == right)
        .count()
}

pub(crate) fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub(crate) fn rfind_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(haystack.len());
    }
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}

pub(crate) fn find_runes(haystack: &[u32], needle: &[u32]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub(crate) fn index_of(bytes: &[u8], pattern: &[u8], start: isize) -> isize {
    if start > bytes.len().saturating_sub(1) as isize {
        return -1;
    }
    let start = start.max(0) as usize;
    find_subslice(&bytes[start..], pattern).map_or(-1, |index| (index + start) as isize)
}

pub(crate) fn last_index_of(bytes: &[u8], pattern: &[u8], start: isize) -> isize {
    if start < 0 {
        return -1;
    }
    if start as usize >= bytes.len() {
        return rfind_subslice(bytes, pattern).map_or(-1, |index| index as isize);
    }
    let (_, width) = decode_go_rune(&bytes[start as usize..]);
    let end = (start as usize + width).min(bytes.len());
    rfind_subslice(&bytes[..end], pattern).map_or(-1, |index| index as isize)
}

pub(crate) fn index_of_runes(target: &[u32], pattern: &[u32], start: isize) -> isize {
    if start > target.len().saturating_sub(1) as isize {
        return -1;
    }
    let start = start.max(0) as usize;
    find_runes(&target[start..], pattern).map_or(-1, |index| (index + start) as isize)
}

pub(crate) fn split_bytes(bytes: &[u8], delimiter: u8) -> Vec<&[u8]> {
    let mut result = Vec::new();
    let mut start = 0;
    for (index, &byte) in bytes.iter().enumerate() {
        if byte == delimiter {
            result.push(&bytes[start..index]);
            start = index + 1;
        }
    }
    result.push(&bytes[start..]);
    result
}

pub(crate) fn query_escape(bytes: &[u8]) -> Vec<u8> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = Vec::with_capacity(bytes.len());
    for &byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            output.push(byte);
        } else if byte == b' ' {
            output.push(b'+');
        } else {
            output.push(b'%');
            output.push(HEX[usize::from(byte >> 4)]);
            output.push(HEX[usize::from(byte & 0x0F)]);
        }
    }
    output
}

pub(crate) fn query_unescape(bytes: &[u8]) -> Result<Vec<u8>, PatchError> {
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            b'%' => {
                if index + 2 >= bytes.len() {
                    return Err(PatchError::InvalidEscape(format!(
                        "invalid URL escape {:?}",
                        String::from_utf8_lossy(&bytes[index..])
                    )));
                }
                let high = hex_value(bytes[index + 1]);
                let low = hex_value(bytes[index + 2]);
                let (Some(high), Some(low)) = (high, low) else {
                    return Err(PatchError::InvalidEscape(format!(
                        "invalid URL escape {:?}",
                        String::from_utf8_lossy(&bytes[index..index + 3])
                    )));
                };
                output.push((high << 4) | low);
                index += 3;
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    Ok(output)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(crate) fn unescape_for_uri(bytes: Vec<u8>) -> Vec<u8> {
    const REPLACEMENTS: &[(&[u8], u8)] = &[
        (b"%21", b'!'),
        (b"%7E", b'~'),
        (b"%27", b'\''),
        (b"%28", b'('),
        (b"%29", b')'),
        (b"%3B", b';'),
        (b"%2F", b'/'),
        (b"%3F", b'?'),
        (b"%3A", b':'),
        (b"%40", b'@'),
        (b"%26", b'&'),
        (b"%3D", b'='),
        (b"%2B", b'+'),
        (b"%24", b'$'),
        (b"%2C", b','),
        (b"%23", b'#'),
        (b"%2A", b'*'),
    ];

    let mut output = bytes;
    for &(pattern, replacement) in REPLACEMENTS {
        let mut next = Vec::with_capacity(output.len());
        let mut index = 0;
        while index < output.len() {
            if output[index..].starts_with(pattern) {
                next.push(replacement);
                index += pattern.len();
            } else {
                next.push(output[index]);
                index += 1;
            }
        }
        output = next;
    }
    output
}

pub(crate) fn escaped_for_patch(bytes: &[u8]) -> Vec<u8> {
    let mut escaped = query_escape(bytes);
    for byte in &mut escaped {
        if *byte == b'+' {
            *byte = b' ';
        }
    }
    unescape_for_uri(escaped)
}

pub(crate) fn html_escape(bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            b'&' => output.extend_from_slice(b"&amp;"),
            b'\'' => output.extend_from_slice(b"&#39;"),
            b'<' => output.extend_from_slice(b"&lt;"),
            b'>' => output.extend_from_slice(b"&gt;"),
            b'\"' => output.extend_from_slice(b"&#34;"),
            _ => output.push(byte),
        }
    }
    output
}

pub(crate) fn string_to_indexes(text: &[u8]) -> Vec<u32> {
    decode_go_runes(text)
        .into_iter()
        .map(|rune| if rune < 0xE000 { rune } else { rune - 0x800 })
        .collect()
}

pub(crate) fn indexes_to_string(indexes: &[u32]) -> Vec<u8> {
    let runes: Vec<u32> = indexes
        .iter()
        .map(|&index| if index < 0xD800 { index } else { index + 0x800 })
        .collect();
    encode_go_runes(&runes)
}

#[cfg(test)]
pub(crate) fn int_to_rune(mut value: u32) -> u32 {
    if value < (1 << 7) {
        return value;
    }
    if value < (1 << 11) {
        return value;
    }
    if value < ((1 << 16) - 0x800 - 3) {
        if value >= 0xD800 {
            value += 0x800;
        }
        assert!(
            char::from_u32(value).is_some() && value != REPLACEMENT,
            "Error encoding an int as a three-byte rune"
        );
        return value;
    }
    if value < ((1 << 21) - 0x800 - 3) {
        let rune = value + 0x800 + 3;
        assert!(
            char::from_u32(rune).is_some() && rune != REPLACEMENT,
            "Error encoding an int as a four-byte rune"
        );
        return rune;
    }
    panic!("The integer {value} is too large for runeToInt()")
}

pub(crate) fn rune_to_int(rune: u32) -> u32 {
    if rune < (1 << 7) {
        return rune;
    }
    if rune < (1 << 11) {
        return rune;
    }
    if rune < (1 << 16) {
        return if rune >= 0xDFFF { rune - 0x800 } else { rune };
    }
    rune - 0x800 - 3
}
