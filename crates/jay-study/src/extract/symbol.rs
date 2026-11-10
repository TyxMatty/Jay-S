//! The Symbol font, as Word and its Equation Editor put it in a PDF: the glyph for code `c` says it
//! is the private-use character U+F000 + `c`, so "μ = 12" comes out as "\u{f06d} \u{f03d} 12". This
//! is Adobe's Symbol encoding, back to the letters it draws.

/// The text a private-use Symbol character stands for: `None` when it is not one this table knows
/// (it stays as it came), `Some("")` for the pieces tall brackets are built from, which add nothing.
// shortcut: other symbol fonts use the same private codes (Wingdings bullets at U+F0A7, U+F0D8,
// U+F0DC, U+F0FC), so the codes that are ornaments or rare in Symbol are left out; a Wingdings
// square bullet at U+F06E still reads as "ν". Tell the fonts apart by name when the PDF library
// exposes it.
pub fn symbol(c: char) -> Option<&'static str> {
    let code = (c as u32).checked_sub(0xF000).filter(|code| (0x20..=0xFF).contains(code))? as u8;
    Some(match code {
        0x20 => " ",
        0x21 => "!",
        0x22 => "∀",
        0x23 => "#",
        0x24 => "∃",
        0x25 => "%",
        0x26 => "&",
        0x27 => "∋",
        0x28 => "(",
        0x29 => ")",
        0x2A => "*",
        0x2B => "+",
        0x2C => ",",
        0x2D => "−",
        0x2E => ".",
        0x2F => "/",
        0x30..=0x39 => &"0123456789"[(code - 0x30) as usize..][..1],
        0x3A => ":",
        0x3B => ";",
        0x3C => "<",
        0x3D => "=",
        0x3E => ">",
        0x3F => "?",
        0x40 => "≅",
        0x41..=0x5A => GREEK_UPPER[(code - 0x41) as usize],
        0x5B => "[",
        0x5C => "∴",
        0x5D => "]",
        0x5E => "⊥",
        0x5F => "_",
        0x61..=0x7A => GREEK_LOWER[(code - 0x61) as usize],
        0x7B => "{",
        0x7C => "|",
        0x7D => "}",
        0x7E => "∼",
        0xA2 => "′",
        0xA3 => "≤",
        0xA4 => "/",
        0xA5 => "∞",
        0xAB => "↔",
        0xAC => "←",
        0xAD => "↑",
        0xAE => "→",
        0xAF => "↓",
        0xB0 => "°",
        0xB1 => "±",
        0xB2 => "″",
        0xB3 => "≥",
        0xB4 => "×",
        0xB5 => "∝",
        0xB6 => "∂",
        0xB7 => "•",
        0xB8 => "÷",
        0xB9 => "≠",
        0xBA => "≡",
        0xBB => "≈",
        0xBC => "…",
        0xC5 => "⊕",
        0xC6 => "∅",
        0xC7 => "∩",
        0xC8 => "∪",
        0xC9 => "⊃",
        0xCA => "⊇",
        0xCB => "⊄",
        0xCC => "⊂",
        0xCD => "⊆",
        0xCE => "∈",
        0xCF => "∉",
        0xD0 => "∠",
        0xD1 => "∇",
        0xD5 => "∏",
        0xD6 => "√",
        0xD7 => "⋅",
        0xD9 => "∧",
        0xDA => "∨",
        0xDB => "⇔",
        0xDE => "⇒",
        0xE5 => "∑",
        0xF2 => "∫",
        // A tall bracket is a top, extensions and a bottom: the top stands for the whole bracket.
        0xE6 => "(",
        0xE9 => "[",
        0xEC => "{",
        0xF6 => ")",
        0xF9 => "]",
        0x60 | 0xBD | 0xBE | 0xE7 | 0xE8 | 0xEA | 0xEB | 0xED..=0xEF | 0xF3..=0xF5 | 0xF7 | 0xF8 | 0xFA | 0xFB => "",
        _ => return None,
    })
}

const GREEK_UPPER: [&str; 26] = ["Α", "Β", "Χ", "Δ", "Ε", "Φ", "Γ", "Η", "Ι", "ϑ", "Κ", "Λ", "Μ", "Ν", "Ο", "Π", "Θ", "Ρ", "Σ", "Τ", "Υ", "ς", "Ω", "Ξ", "Ψ", "Ζ"];
const GREEK_LOWER: [&str; 26] = ["α", "β", "χ", "δ", "ε", "φ", "γ", "η", "ι", "ϕ", "κ", "λ", "μ", "ν", "ο", "π", "θ", "ρ", "σ", "τ", "υ", "ϖ", "ω", "ξ", "ψ", "ζ"];

#[cfg(test)]
mod tests {
    use super::symbol;

    #[test]
    fn symbol_codes_read_as_the_letters_they_draw() {
        let read = |s: &str| s.chars().map(|c| symbol(c).map(str::to_owned).unwrap_or_else(|| c.to_string())).collect::<String>();
        // What a statistics lecture has: "μ = 12", "(1 − α)", "σ ≤ ∞", "x ± 3".
        assert_eq!(read("\u{f06d} \u{f03d} 12"), "μ = 12");
        assert_eq!(read("(1 \u{f02d} \u{f061})"), "(1 − α)");
        assert_eq!(read("\u{f073} \u{f0a3} \u{f0a5} \u{f0b1} \u{f0b3} \u{f0de} \u{f070} \u{f053} \u{f044}"), "σ ≤ ∞ ± ≥ ⇒ π Σ Δ");
        assert_eq!(read("\u{f031}\u{f039}"), "19");
        // A tall bracket in three pieces is one bracket.
        assert_eq!(read("\u{f0e6}\u{f0e7}\u{f0e8}x\u{f0f6}\u{f0f7}\u{f0f8}"), "(x)");
        // Ordinary text, other private-use characters and the ornament codes stay as they came.
        for c in ['a', 'μ', '\u{e000}', '\u{f000}', '\u{f01f}', '\u{f100}', '\u{f0a7}', '\u{f0d2}', '\u{f0d8}', '\u{f0dc}', '\u{f0fc}', '\u{f0ff}', '\u{10ffff}'] {
            assert_eq!(symbol(c), None, "{c:?}");
        }
        // Every code answers without a panic.
        assert_eq!((0xF000..=0xF0FFu32).filter(|&c| symbol(char::from_u32(c).unwrap()).is_some()).count(), 160);
    }
}
