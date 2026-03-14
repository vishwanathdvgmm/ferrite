pub const MATHUTILS: &str = include_str!("mathutils.fe");
pub const STRINGS: &str = include_str!("strings.fe");
pub const COLLECTIONS: &str = include_str!("collections.fe");
pub const FUNCTIONAL: &str = include_str!("functional.fe");

pub fn get_stdlib_module(name: &str) -> Option<&'static str> {
    match name {
        "std/mathutils" | "mathutils" => Some(MATHUTILS),
        "std/strings" | "strings" => Some(STRINGS),
        "std/collections" | "collections" => Some(COLLECTIONS),
        "std/functional" | "functional" => Some(FUNCTIONAL),
        _ => None,
    }
}
