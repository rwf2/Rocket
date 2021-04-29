pub enum SchemaKind {
    Null,
    Map,
    List,
    String,
    Num,
    Int,
    Bool,
    Set,
}

pub struct TypeSchema<T> {
    pub description: Option<String>,
    pub example: Option<T>,
    pub name: String,
    pub kind: SchemaKind,

}

pub trait HasSchema: Sized {
    fn schema() -> TypeSchema<Self>;
}

// impls for the entire serde data model:

// 14 primitve types
impl HasSchema for i8 {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some(1),
            name: "signed 8-bits integer".to_string(),
            kind: SchemaKind::Int,
        }
    }
}

impl HasSchema for i16 {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some(1),
            name: "signed 16-bits integer".to_string(),
            kind: SchemaKind::Int,
        }
    }
}

impl HasSchema for i32 {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some(1),
            name: "signed 32-bits integer".to_string(),
            kind: SchemaKind::Int,
        }
    }
}

impl HasSchema for i64 {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some(1),
            name: "signed 64-bits integer".to_string(),
            kind: SchemaKind::Int,
        }
    }
}

impl HasSchema for i128 {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some(1),
            name: "signed 128-bits integer".to_string(),
            kind: SchemaKind::Int,
        }
    }
}

impl HasSchema for u8 {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some(1),
            name: "unsigned 8-bits integer".to_string(),
            kind: SchemaKind::Int,
        }
    }
}

impl HasSchema for u16 {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some(1),
            name: "unsigned 16-bits integer".to_string(),
            kind: SchemaKind::Int,
        }
    }
}

impl HasSchema for u32 {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some(1),
            name: "unsigned 32-bits integer".to_string(),
            kind: SchemaKind::Int,
        }
    }
}

impl HasSchema for u64 {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some(1),
            name: "unsigned 64-bits integer".to_string(),
            kind: SchemaKind::Int,
        }
    }
}

impl HasSchema for u128 {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some(1),
            name: "unsigned 128-bits integer".to_string(),
            kind: SchemaKind::Int,
        }
    }
}

impl HasSchema for f32 {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some(1.0),
            name: "32-bits floating point".to_string(),
            kind: SchemaKind::Num,
        }
    }
}

impl HasSchema for f64 {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some(1.0),
            name: "64-bits floating point".to_string(),
            kind: SchemaKind::Num,
        }
    }
}

impl HasSchema for bool {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some(true),
            name: "boolean".to_string(),
            kind: SchemaKind::Bool,
        }
    }
}

// string
impl<'a> HasSchema for &'a str {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some("string"),
            name: "signed 8-bits integer".to_string(),
            kind: SchemaKind::String,
        }
    }
}

impl<'a> HasSchema for String {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some("string".to_string()),
            name: "signed 8-bits integer".to_string(),
            kind: SchemaKind::String,
        }
    }
}

// byte array
impl<'a> HasSchema for &'a [u8] {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: None,
            name: "An array of bytes".to_string(),
            kind: SchemaKind::List,
        }
    }
}

// option
impl<T: HasSchema> HasSchema for Option<T> {
    fn schema() -> TypeSchema<Self> {
        let base_schema = T::schema();
        TypeSchema {
            description: None,
            example: Some(base_schema.example),
            name: format!("Optional: {}", base_schema.name),
            kind: base_schema.kind,
        }
    }
}

// unit
impl HasSchema for () {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: Some(()),
            name: "Nothing".to_string(),
            kind: SchemaKind::Null,
        }
    }
}

// seq
impl<T: HasSchema, const N: usize> HasSchema for [T; N] {
    fn schema() -> TypeSchema<Self> {
        let base_schema = T::schema();
        TypeSchema {
            description: None,
            example: None, // making an array example requires that T be Copy...
            name: format!("Array of {} {}'s", N, base_schema.name),
            kind: SchemaKind::List,
        }
    }
}

impl<T: HasSchema> HasSchema for Vec<T> {
    fn schema() -> TypeSchema<Self> {
        let base_schema = T::schema();
        TypeSchema {
            description: None,
            example: None, // making an array example requires that T be Copy...
            name: format!("Unsized array of {}'s", base_schema.name),
            kind: SchemaKind::List,
        }
    }
}

impl<T: HasSchema> HasSchema for std::collections::HashSet<T> {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: None, // making an array example requires that T be Copy...
            name: format!("Set of {}'s", T::schema().name),
            kind: SchemaKind::Set,
        }
    }
}

// tuple
impl<T1: HasSchema> HasSchema for (T1, ) {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: None, // making an array example requires that T be Copy...
            name: format!("Unary tuple of an {}", T1::schema().name),
            kind: SchemaKind::Set,
        }
    }
}

impl<T1: HasSchema, T2: HasSchema> HasSchema for (T1, T2) {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: None, // making an array example requires that T be Copy...
            name: format!("Tuple of the form ({}, {})", T1::schema().name, T2::schema().name),
            kind: SchemaKind::Set,
        }
    }
}

// todo: extend with macros

// map
impl<K: HasSchema, V: HasSchema> HasSchema for std::collections::HashMap<K, V> {
    fn schema() -> TypeSchema<Self> {
        TypeSchema {
            description: None,
            example: None, // making an array example requires that T be Copy...
            name: format!("Map from {} to {}", K::schema().name, V::schema().name),
            kind: SchemaKind::Map,
        }
    }
}



// impl<T: HasSchema> HasSchema for Box<T> {
//     fn schema() -> TypeSchema<Self> {
//         let base_schema = T::schema();
//         TypeSchema {
//             description: base_schema.description,
//             example: base_schema.example.map(Box::new),
//             name: base_schema.name,
//             kind: base_schema.kind,
//         }
//     }
// }



