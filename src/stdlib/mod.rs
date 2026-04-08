pub const MATH_FE: &str = include_str!("math.fe");
pub const STRINGS_FE: &str = include_str!("strings.fe");
pub const COLLECTIONS_FE: &str = include_str!("collections.fe");
pub const IO_FE: &str = include_str!("io.fe");

pub fn get_stdlib_module(name: &str) -> Option<&'static str> {
    match name {
        "math" => Some(MATH_FE),
        "strings" => Some(STRINGS_FE),
        "collections" => Some(COLLECTIONS_FE),
        "io" => Some(IO_FE),
        _ => None,
    }
}
