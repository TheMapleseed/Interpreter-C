pub enum CType {
    Void,
    Char { signed: bool },
    Short { signed: bool },
    Int { signed: bool },
    Long { signed: bool },
    LongLong { signed: bool },
    Float,
    Double,
    LongDouble,
    Pointer(Box<CType>),
    Array(Box<CType>, Option<usize>),
    Struct(StructType),
    Union(UnionType),
    Enum(EnumType),
    Function(FunctionType),
    Typedef(String),
} 