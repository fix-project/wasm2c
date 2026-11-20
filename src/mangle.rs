// name mangling

// mostly follows wabt (desc pasted below)

// Name mangling transforms arbitrary Wasm names into "safe" C++ names
// in a deterministic way. To avoid collisions, distinct Wasm names must be
// transformed into distinct C++ names.
//
// The rules implemented here are:
// 1) any hex digit (0-9, A-F) that follows the sequence "0x"
//    is escaped
// 2) any underscore at the beginning, at the end, or following another
//    underscore, is escaped
// 3) otherwise, any alphanumeric character (or underscore) is kept as-is,
//    and any other character is escaped
//
// "Escaped" means the character is represented with the sequence "0xAB",
// where A B are hex digits (0-9, A-F) representing the character's
// numeric value.

pub(crate) fn mangle(name: &str) -> String {
    // tracks the format of a hex digit
    #[derive(PartialEq, Eq)]
    enum HexState {
        None,
        Zero,
        ZeroX,
    }

    let bytes = name.as_bytes();
    let mut out = String::new();
    let mut state = HexState::None;
    let mut last_was_underscore = false;

    for (i, &ch) in bytes.iter().enumerate() {
        // RULE 1
        // any hex digit (0-9, A-F) that follows the sequence "0x" is escaped
        let is_hex = state == HexState::ZeroX && ch.is_ascii_hexdigit();

        // track hex digit
        state = match (state, ch) {
            (HexState::Zero, b'x') => HexState::ZeroX,
            (_, b'0') => HexState::Zero,
            _ => HexState::None,
        };

        // RULE 2
        // bad_underscore = this is _ AND (this is start / this is end / last was underscore)
        // i.e. underscore at start, underscore at end, or double underscore
        let is_bad_underscore =
            ch == b'_' && (i == 0 || i == bytes.len() - 1 || last_was_underscore);

        // RULE 3
        // characters that are not alphanumeric or a valid underscore is escaped
        let is_valid_ch = ch.is_ascii_alphanumeric() || ch == b'_';

        // ESCAPE
        if is_hex || is_bad_underscore || !is_valid_ch {
            // escaped version of the character
            out.push_str(&format!("0x{ch:02X}"));
            state = HexState::None;
            last_was_underscore = false;
        }
        // VALID
        else {
            out.push(ch as char);
            last_was_underscore = ch == b'_';
        }
    }
    out
}
