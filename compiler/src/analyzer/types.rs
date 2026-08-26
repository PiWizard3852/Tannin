use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntWidth {
    Bits(u32),
    Size,
}

impl IntWidth {
    pub(crate) fn bits(self) -> u32 {
        match self {
            IntWidth::Bits(bits) => bits,
            IntWidth::Size => usize::BITS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ValueType {
    Integer { width: IntWidth, signed: bool },
    Float(u32),
    Bool,
    Char,
    String,
    Struct(String),
    Array(Box<ValueType>),
}

impl ValueType {
    pub(crate) const ISIZE: Self = Self::Integer {
        width: IntWidth::Size,
        signed: true,
    };

    pub(crate) fn named(name: &str) -> Option<Self> {
        let (width, signed) = match name {
            "i8" => (IntWidth::Bits(8), true),
            "i16" => (IntWidth::Bits(16), true),
            "i32" => (IntWidth::Bits(32), true),
            "i64" => (IntWidth::Bits(64), true),
            "isize" => (IntWidth::Size, true),
            "u8" => (IntWidth::Bits(8), false),
            "u16" => (IntWidth::Bits(16), false),
            "u32" => (IntWidth::Bits(32), false),
            "u64" => (IntWidth::Bits(64), false),
            "usize" => (IntWidth::Size, false),
            "f32" => return Some(Self::Float(32)),
            "f64" => return Some(Self::Float(64)),
            "bool" => return Some(Self::Bool),
            "char" => return Some(Self::Char),
            "string" => return Some(Self::String),
            _ => return None,
        };

        Some(Self::Integer { width, signed })
    }

    pub(crate) fn is_integer(&self) -> bool {
        matches!(self, Self::Integer { .. })
    }

    pub(crate) fn is_numeric(&self) -> bool {
        self.is_integer() || matches!(self, Self::Float(_))
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer { width, signed } => {
                let prefix = if *signed { "i" } else { "u" };

                match width {
                    IntWidth::Bits(bits) => write!(f, "{prefix}{bits}"),
                    IntWidth::Size => write!(f, "{prefix}size"),
                }
            }

            Self::Float(bits) => write!(f, "f{bits}"),
            Self::Bool => write!(f, "bool"),
            Self::Char => write!(f, "char"),
            Self::String => write!(f, "string"),
            Self::Struct(name) => write!(f, "{name}"),
            Self::Array(element) => write!(f, "[{element}]"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FunctionType {
    pub parameters: Vec<ValueType>,
    pub return_type: Option<ValueType>,
}
