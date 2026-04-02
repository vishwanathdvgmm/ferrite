use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ShapeDim {
    Const(i64),
    Symbolic(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TensorShape {
    pub dims: Vec<ShapeDim>,
}

impl TensorShape {
    pub fn new(dims: Vec<ShapeDim>) -> Self {
        Self { dims }
    }

    /// Check if two shapes exactly match (no implicit broadcasting allowed)
    pub fn exact_match(&self, other: &Self) -> bool {
        if self.dims.len() != other.dims.len() {
            return false;
        }

        for (a, b) in self.dims.iter().zip(other.dims.iter()) {
            if a != b {
                return false; // Symbolic or const mismatch
            }
        }

        true
    }
}

impl fmt::Display for TensorShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        for (i, dim) in self.dims.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            match dim {
                ShapeDim::Const(n) => write!(f, "{}", n)?,
                ShapeDim::Symbolic(s) => write!(f, "{}", s)?,
            }
        }
        write!(f, ")")
    }
}
