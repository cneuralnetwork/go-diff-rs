use crate::util::{indexes_to_string, string_to_indexes};

#[test]
fn test_index_conversion() {
    let count = 0x11_0000 - (0xE000 - 0xD800);
    let indexes: Vec<u32> = (0..count).collect();
    let converted = string_to_indexes(&indexes_to_string(&indexes));
    assert_eq!(indexes, converted);
}
