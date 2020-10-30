use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::From;
use std::rc::Rc;

trait TypeSize {
    /// Returns the size (in bytes) of this type
    fn type_size(&self) -> usize;
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Type {
    Class(Class),
    Union(Union),
    Bitfield(Bitfield),
    Enumeration(Enumeration),
    EnumVariant(EnumVariant),
    Pointer(Pointer),
    Primitive(Primitive),
    Array(Array),
    FieldList(FieldList),
    Modifier(Modifier),
    Member(Member),
}

impl TypeSize for Type {
    fn type_size(&self) -> usize {
        match self {
            Type::Class(class) => class.size,
            Type::Union(union) => union.size,
            Type::Bitfield(bitfield) => bitfield.underlying_type.type_size(),
            Type::Enumeration(e) => e.underlying_type.type_size(),
            Type::Pointer(p) => p.underlying_type.type_size(),
            Type::Primitive(p) => p.type_size(),
            Type::Array(a) => a.size,
            Type::FieldList(fields) => fields
                .0
                .iter()
                .fold(0, |acc, field| acc + field.type_size()),
            Type::EnumVariant(_) => panic!("type_size() invoked for EnumVariant"),
            Type::Modifier(_) => panic!("type_size() invoked for Modifier"),
            Type::Member(_) => panic!("type_size() invoked for Modifier"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Class {
    name: String,
    unique_name: Option<String>,
    kind: ClassKind,
    fields: Vec<Rc<Type>>,
    size: usize,
}

type FromClass<'a, 'b> = (
    &'b pdb::ClassType<'a>,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromClass<'_, '_>> for Class {
    fn from(info: FromClass<'_, '_>) -> Self {
        let (class, type_finder, output_pdb) = info;

        let pdb::ClassType {
            kind,
            count,
            properties,
            fields,
            derived_from,
            vtable_shape,
            size,
            name,
            unique_name,
        } = *class;

        let fields: Vec<Rc<Type>> = fields
            .map(|type_index| {
                // TODO: perhaps change FieldList to Rc<Vec<Rc<Type>>?
                if let Type::FieldList(fields) =
                    crate::parse::handle_type(type_index, output_pdb, type_finder)
                        .expect("failed to resolve dependent type")
                        .as_ref()
                {
                    fields.0.clone()
                } else {
                    panic!("got an unexpected type when FieldList was expected")
                }
            })
            .unwrap_or_default();

        Class {
            name: name.to_string().into_owned(),
            unique_name: unique_name.map(|s| s.to_string().into_owned()),
            kind: kind.into(),
            fields,
            size: size as usize,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClassKind {
    Class,
    Struct,
    Interface,
}

impl From<pdb::ClassKind> for ClassKind {
    fn from(kind: pdb::ClassKind) -> Self {
        match kind {
            pdb::ClassKind::Class => ClassKind::Class,
            pdb::ClassKind::Struct => ClassKind::Struct,
            pdb::ClassKind::Interface => ClassKind::Interface,
        }
    }
}

type FromUnion<'a, 'b> = (
    &'b pdb::UnionType<'a>,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);
#[derive(Debug, Serialize, Deserialize)]
pub struct Union {
    name: String,
    unique_name: Option<String>,
    size: usize,
    count: usize,
    fields: Vec<Rc<Type>>,
}
impl From<FromUnion<'_, '_>> for Union {
    fn from(data: FromUnion<'_, '_>) -> Self {
        let (union, type_finder, parsed_pdb) = data;
        let pdb::UnionType {
            count,
            properties,
            size,
            fields,
            name,
            unique_name,
        } = *union;

        // TODO: perhaps change FieldList to Rc<Vec<Rc<Type>>?
        let fields = if let Type::FieldList(fields) =
            crate::parse::handle_type(fields, parsed_pdb, type_finder)
                .expect("failed to resolve dependent type")
                .as_ref()
        {
            fields.0.clone()
        } else {
            panic!("got an unexpected type when FieldList was expected")
        };

        Union {
            name: name.to_string().into_owned(),
            unique_name: unique_name.map(|s| s.to_string().into_owned()),
            size: size as usize,
            count: count as usize,
            fields: vec![],
        }
    }
}

type FromBitfield<'a, 'b> = (
    &'b pdb::BitfieldType,
    &'b pdb::TypeFinder<'a>,
    &'b HashMap<u32, Rc<Type>>,
);
#[derive(Debug, Serialize, Deserialize)]
pub struct Bitfield {
    underlying_type: Rc<Type>,
    len: usize,
    position: usize,
}
impl From<FromBitfield<'_, '_>> for Bitfield {
    fn from(data: FromBitfield<'_, '_>) -> Self {
        let (bitfield, type_finder, parsed_types) = data;
        let pdb::BitfieldType {
            underlying_type,
            length,
            position,
        } = *bitfield;

        let underlying_type = match parsed_types.get(&underlying_type.0) {
            Some(typ) => Rc::clone(typ),
            None => panic!("dependent type has not yet been parsed"),
        };

        Bitfield {
            underlying_type,
            len: length as usize,
            position: position as usize,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Enumeration {
    name: String,
    unique_name: Option<String>,
    underlying_type: Rc<Type>,
    variants: Vec<EnumVariant>,
}

type FromEnumeration<'a, 'b> = (
    &'b pdb::EnumerationType<'a>,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromEnumeration<'_, '_>> for Enumeration {
    fn from(data: FromEnumeration<'_, '_>) -> Self {
        let (e, type_finder, output_pdb) = data;

        let pdb::EnumerationType {
            count,
            properties,
            underlying_type,
            fields,
            name,
            unique_name,
        } = e;

        let underlying_type = crate::parse::handle_type(*underlying_type, output_pdb, type_finder)
            .expect("failed to resolve underlying type");
        // TODO: Variants

        Enumeration {
            name: name.to_string().into_owned(),
            unique_name: unique_name.map(|s| s.to_string().into_owned()),
            underlying_type,
            variants: vec![],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnumVariant {
    name: String,
    value: VariantValue,
}

type FromEnumerate<'a, 'b> = &'b pdb::EnumerateType<'a>;

impl From<FromEnumerate<'_, '_>> for EnumVariant {
    fn from(data: FromEnumerate<'_, '_>) -> Self {
        let e = data;

        let pdb::EnumerateType {
            attributes,
            value,
            name,
        } = e;

        Self {
            name: name.to_string().into_owned(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum VariantValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
}

type FromVariant = pdb::Variant;

impl From<&FromVariant> for VariantValue {
    fn from(data: &FromVariant) -> Self {
        let variant = data;

        match *variant {
            pdb::Variant::U8(val) => VariantValue::U8(val),
            pdb::Variant::U16(val) => VariantValue::U16(val),
            pdb::Variant::U32(val) => VariantValue::U32(val),
            pdb::Variant::U64(val) => VariantValue::U64(val),
            pdb::Variant::I8(val) => VariantValue::I8(val),
            pdb::Variant::I16(val) => VariantValue::I16(val),
            pdb::Variant::I32(val) => VariantValue::I32(val),
            pdb::Variant::I64(val) => VariantValue::I64(val),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pointer {
    // TODO: we don't know the width of the pointer
    underlying_type: Rc<Type>,
    attributes: PointerAttributes,
}

type FromPointer<'a, 'b> = (
    &'b pdb::PointerType,
    &'b pdb::TypeFinder<'a>,
    &'b HashMap<u32, Rc<Type>>,
);
impl From<FromPointer<'_, '_>> for Pointer {
    fn from(data: FromPointer<'_, '_>) -> Self {
        let (pointer, type_finder, parsed_types) = data;
        let pdb::PointerType {
            underlying_type,
            attributes,
            containing_class,
        } = *pointer;

        let underlying_type = match parsed_types.get(&underlying_type.0) {
            Some(typ) => Rc::clone(typ),
            None => panic!("dependent type has not yet been parsed"),
        };

        Pointer {
            underlying_type,
            attributes: attributes.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PointerAttributes {
    is_volatile: bool,
    is_const: bool,
    is_unaligned: bool,
    is_restrict: bool,
    is_reference: bool,
    size: usize,
    is_mocom: bool,
}

impl From<pdb::PointerAttributes> for PointerAttributes {
    fn from(attr: pdb::PointerAttributes) -> Self {
        PointerAttributes {
            is_volatile: attr.is_volatile(),
            is_const: attr.is_const(),
            is_unaligned: attr.is_unaligned(),
            is_restrict: attr.is_restrict(),
            is_reference: attr.is_reference(),
            size: attr.size() as usize,
            is_mocom: attr.is_mocom(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Primitive {
    NoType,
    Void,
    Char,
    UChar,
    RChar,
    WChar,
    RChar16,
    RChar32,
    I8,
    U8,
    Short,
    UShort,
    I16,
    U16,
    Long,
    ULong,
    I32,
    U32,
    Quad,
    UQuad,
    I64,
    U64,
    Octa,
    UOcta,
    I128,
    U128,
    F16,
    F32,
    F32PP,
    F48,
    F64,
    F80,
    F128,
    Complex32,
    Complex64,
    Complex80,
    Complex128,
    Bool8,
    Bool16,
    Bool32,
    Bool64,
    HRESULT,
}

impl From<&pdb::PrimitiveType> for Primitive {
    fn from(typ: &pdb::PrimitiveType) -> Self {
        let pdb::PrimitiveType { kind, indirection } = typ;

        match *kind {
            pdb::PrimitiveKind::NoType => Primitive::NoType,
            pdb::PrimitiveKind::Void => Primitive::Void,
            pdb::PrimitiveKind::Char => Primitive::Char,
            pdb::PrimitiveKind::UChar => Primitive::UChar,
            pdb::PrimitiveKind::RChar => Primitive::RChar,
            pdb::PrimitiveKind::WChar => Primitive::WChar,
            pdb::PrimitiveKind::RChar16 => Primitive::RChar16,
            pdb::PrimitiveKind::RChar32 => Primitive::RChar32,
            pdb::PrimitiveKind::I8 => Primitive::I8,
            pdb::PrimitiveKind::U8 => Primitive::U8,
            pdb::PrimitiveKind::Short => Primitive::Short,
            pdb::PrimitiveKind::UShort => Primitive::UShort,
            pdb::PrimitiveKind::I16 => Primitive::I16,
            pdb::PrimitiveKind::U16 => Primitive::U16,
            pdb::PrimitiveKind::Long => Primitive::Long,
            pdb::PrimitiveKind::ULong => Primitive::ULong,
            pdb::PrimitiveKind::I32 => Primitive::I32,
            pdb::PrimitiveKind::U32 => Primitive::U32,
            pdb::PrimitiveKind::Quad => Primitive::Quad,
            pdb::PrimitiveKind::UQuad => Primitive::UQuad,
            pdb::PrimitiveKind::I64 => Primitive::I64,
            pdb::PrimitiveKind::U64 => Primitive::U64,
            pdb::PrimitiveKind::Octa => Primitive::Octa,
            pdb::PrimitiveKind::UOcta => Primitive::UOcta,
            pdb::PrimitiveKind::I128 => Primitive::I128,
            pdb::PrimitiveKind::U128 => Primitive::U128,
            pdb::PrimitiveKind::F16 => Primitive::F16,
            pdb::PrimitiveKind::F32 => Primitive::F32,
            pdb::PrimitiveKind::F32PP => Primitive::F32PP,
            pdb::PrimitiveKind::F48 => Primitive::F48,
            pdb::PrimitiveKind::F64 => Primitive::F64,
            pdb::PrimitiveKind::F80 => Primitive::F80,
            pdb::PrimitiveKind::F128 => Primitive::F128,
            pdb::PrimitiveKind::Complex32 => Primitive::Complex32,
            pdb::PrimitiveKind::Complex64 => Primitive::Complex64,
            pdb::PrimitiveKind::Complex80 => Primitive::Complex80,
            pdb::PrimitiveKind::Complex128 => Primitive::Complex128,
            pdb::PrimitiveKind::Bool8 => Primitive::Bool8,
            pdb::PrimitiveKind::Bool16 => Primitive::Bool16,
            pdb::PrimitiveKind::Bool32 => Primitive::Bool32,
            pdb::PrimitiveKind::Bool64 => Primitive::Bool64,
            pdb::PrimitiveKind::HRESULT => Primitive::HRESULT,
        }
    }
}

impl TypeSize for Primitive {
    fn type_size(&self) -> usize {
        match self {
            Primitive::NoType | Primitive::Void => 0,

            Primitive::Char
            | Primitive::UChar
            | Primitive::RChar
            | Primitive::I8
            | Primitive::U8
            | Primitive::Bool8 => 1,

            Primitive::Short
            | Primitive::UShort
            | Primitive::I16
            | Primitive::U16
            | Primitive::F16
            | Primitive::Bool16 => 2,

            Primitive::Long
            | Primitive::ULong
            | Primitive::I32
            | Primitive::U32
            | Primitive::F32
            | Primitive::F32PP
            | Primitive::Bool32
            | Primitive::HRESULT => 4,

            Primitive::Quad
            | Primitive::UQuad
            | Primitive::I64
            | Primitive::U64
            | Primitive::F64
            | Primitive::Bool32 => 8,
            Primitive::Octa | Primitive::UOcta | Primitive::I128 | Primitive::U128 => 16,
            _ => panic!("type size not handled for type: {:?}", self),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Array {
    element_type: Rc<Type>,
    indexing_type: Rc<Type>,
    stride: Option<u32>,
    size: usize,
    dimensions_bytes: Vec<usize>,
    dimensions_elements: Vec<usize>,
}

type FromArray<'a, 'b> = (
    &'b pdb::ArrayType,
    &'b pdb::TypeFinder<'a>,
    &'b HashMap<u32, Rc<Type>>,
);

impl From<FromArray<'_, '_>> for Array {
    fn from(data: FromArray<'_, '_>) -> Self {
        let (array, type_finder, parsed_types) = data;

        let pdb::ArrayType {
            element_type,
            indexing_type,
            stride,
            dimensions,
        } = array;

        let element_type = match parsed_types.get(&element_type.0) {
            Some(typ) => Rc::clone(typ),
            None => panic!("dependent type has not yet been parsed"),
        };

        let indexing_type = match parsed_types.get(&indexing_type.0) {
            Some(typ) => Rc::clone(typ),
            None => panic!("dependent type has not yet been parsed"),
        };

        let size = *dimensions.last().unwrap() as usize;
        let mut last_element_size = element_type.type_size();
        let mut dimensions_elements = vec![];
        println!("{:?}", dimensions);
        for bytes in dimensions {
            let elements = (*bytes as usize) / last_element_size;
            dimensions_elements.push(elements);
            last_element_size = *bytes as usize;
        }

        Array {
            element_type,
            indexing_type,
            stride: *stride,
            size,
            dimensions_bytes: dimensions.iter().map(|b| *b as usize).collect(),
            dimensions_elements,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldList(Vec<Rc<Type>>);

type FromFieldList<'a, 'b> = (
    &'b pdb::FieldList<'b>,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromFieldList<'_, '_>> for FieldList {
    fn from(data: FromFieldList<'_, '_>) -> Self {
        let (fields, type_finder, output_pdb) = data;

        let pdb::FieldList {
            fields,
            continuation,
        } = fields;

        let mut result_fields: Vec<Rc<Type>> = fields
            .iter()
            .map(|typ| {
                crate::parse::handle_type_data(typ, output_pdb, type_finder)
                    .ok()
                    .unwrap_or_else(|| panic!("failed to parse dependent type"))
            })
            .collect();

        if let Some(continuation) = continuation {
            let field = crate::parse::handle_type(*continuation, output_pdb, type_finder)
                .expect("failed to parse dependent type");
            if let Type::FieldList(fields) = field.as_ref() {
                result_fields.append(&mut fields.0.clone())
            } else {
                panic!(
                    "unexpected type returned while getting FieldList continuation: {:?}",
                    field
                )
            }
        }

        FieldList(result_fields)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Modifier {
    underlying_type: Rc<Type>,
    constant: bool,
    volatile: bool,
    unaligned: bool,
}

type FromModifier<'a, 'b> = (
    &'b pdb::ModifierType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromModifier<'_, '_>> for Modifier {
    fn from(data: FromModifier<'_, '_>) -> Self {
        let (modifier, type_finder, output_pdb) = data;

        let pdb::ModifierType {
            underlying_type,
            constant,
            volatile,
            unaligned,
        } = *modifier;

        let underlying_type = crate::parse::handle_type(underlying_type, output_pdb, type_finder)
            .expect("failed to parse dependent type");

        Modifier {
            underlying_type,
            constant,
            volatile,
            unaligned,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Member {
    name: String,
    underlying_type: Rc<Type>,
    offset: usize,
}

type FromMember<'a, 'b> = (
    &'b pdb::MemberType<'a>,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromMember<'_, '_>> for Member {
    fn from(data: FromMember<'_, '_>) -> Self {
        let (member, type_finder, output_pdb) = data;

        let pdb::MemberType {
            attributes,
            field_type,
            offset,
            name,
        } = *member;

        let underlying_type = crate::parse::handle_type(field_type, output_pdb, type_finder)
            .expect("failed to parse dependent type");

        Member {
            name: name.to_string().into_owned(),
            underlying_type,
            offset: offset as usize,
        }
    }
}
