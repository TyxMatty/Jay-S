//! Item ids: 32 random lowercase hex characters. An id is the only thing the page sends back to name
//! a material, an exam or a deck, and it becomes part of a path, so every command checks it with
//! `is_valid` first (docs/ARQUITETURA.md, section 7, item 2).

/// A new random id.
pub fn new_id() -> String {
    hex(&random_bytes::<16>())
}

/// `n` random bytes from the OS.
pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).expect("the OS random source failed");
    bytes
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Exactly 32 lowercase hex characters: no separators, dots, drive letters or reserved names can get
/// through.
pub fn is_valid(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_ids_are_valid_and_differ() {
        let (a, b) = (new_id(), new_id());
        assert!(is_valid(&a) && is_valid(&b));
        assert_ne!(a, b);
    }

    #[test]
    fn hostile_ids_are_refused() {
        for id in [
            "",
            "..",
            "../../../../Windows/System32/aaaa",
            "..\\..\\0123456789abcdef0123456789ab",
            "C:\\Users\\x\\0123456789abcdef0123",
            "\\\\?\\C:\\0123456789abcdef01234567",
            "CON",
            "0123456789ABCDEF0123456789ABCDEF",
            "0123456789abcdef0123456789abcde",
            "0123456789abcdef0123456789abcdef0",
            "0123456789abcdef0123456789abcde/",
            "0123456789abcdef0123456789abcd\0f",
        ] {
            assert!(!is_valid(id), "{id:?}");
        }
        assert!(is_valid("0123456789abcdef0123456789abcdef"));
    }
}
