// name mangling

// follows wabt exactly (desc pasted below)

// Name mangling transforms arbitrary Wasm names into "safe" C names
// in a deterministic way. To avoid collisions, distinct Wasm names must be
// transformed into distinct C names.
//
// The rules implemented here are:
// 1) any hex digit ('A' through 'F') that follows the sequence "0x"
//     is escaped
// 2) any underscore at the beginning, at the end, or following another
//     underscore, is escaped
// 3) if double_underscores is set, underscores are replaced with
//    two underscores.
// 4) otherwise, any alphanumeric character is kept as-is,
//    and any other character is escaped
//
// "Escaped" means the character is represented with the sequence "0xAB",
// where A B are hex digits ('0'-'9' or 'A'-'F') representing the character's
// numeric value.
//
// Module names are mangled with double_underscores=true to prevent
// collisions between, e.g., a module "alfa" with export
// "bravo_charlie" vs. a module "alfa_bravo" with export "charlie".

pub(crate) fn mangle(name: &str) -> String {
    todo!()
}
