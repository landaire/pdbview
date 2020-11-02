use crate::symbol_types::ParsedPdb;
use crate::symbol_types::TypeRef;
use serde::{Deserialize, Serialize};
use std::convert::From;
use std::rc::Rc;

pub trait Typed {
    /// Returns the size (in bytes) of this type
    fn type_size(&self, pdb: &ParsedPdb) -> usize;

    /// Called after all types have been parsed
    fn on_complete(&mut self, pdb: &ParsedPdb) {}
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Type {
    Class(Class),
    VirtualBaseClass(VirtualBaseClass),
    Union(Union),
    Bitfield(Bitfield),
    Enumeration(Enumeration),
    EnumVariant(EnumVariant),
    Pointer(Pointer),
    Primitive(Primitive),
    Array(Array),
    FieldList(FieldList),
    ArgumentList(ArgumentList),
    Modifier(Modifier),
    Member(Member),
    Procedure(Procedure),
    MemberFunction(MemberFunction),
    MethodList(MethodList),
    MethodListEntry(MethodListEntry),
    Nested(Nested),
    OverloadedMethod(OverloadedMethod),
    Method(Method),
    StaticMember(StaticMember),
    BaseClass(BaseClass),
    VTable(VTable),
}

impl Typed for Type {
    fn type_size(&self, pdb: &ParsedPdb) -> usize {
        match self {
            Type::Class(class) => class.type_size(pdb),
            Type::Union(union) => union.type_size(pdb),
            Type::Bitfield(bitfield) => bitfield.underlying_type.borrow().type_size(pdb),
            Type::Enumeration(e) => e.underlying_type.borrow().type_size(pdb),
            Type::Pointer(p) => p.attributes.kind.type_size(pdb),
            Type::Primitive(p) => p.type_size(pdb),
            Type::Array(a) => a.type_size(pdb),
            Type::FieldList(fields) => fields
                .0
                .iter()
                .fold(0, |acc, field| acc + field.borrow().type_size(pdb)),
            Type::EnumVariant(_) => panic!("type_size() invoked for EnumVariant"),
            Type::Modifier(modifier) => modifier.underlying_type.borrow().type_size(pdb),
            Type::Member(_) => panic!("type_size() invoked for Member"),
            Type::ArgumentList(_) => panic!("type_size() invoked for ArgumentList"),
            Type::Procedure(_) => panic!("type_size() invoked for Procedure"),
            Type::MemberFunction(_) => panic!("type_size() invoked for MemberFunction"),
            Type::MethodList(_) => panic!("type_size() invoked for MethodList"),
            Type::MethodListEntry(_) => panic!("type_size() invoked for MethodListEntry"),
            Type::VirtualBaseClass(_) => panic!("type_size() invoked for VirtualBaseClass"),
            Type::Nested(_) => panic!("type_size() invoked for Nested"),
            Type::OverloadedMethod(_) => panic!("type_size() invoked for overloaded method"),
            Type::Method(_) => panic!("type_size() invoked for overloaded method"),
            Type::StaticMember(_) => panic!("type_size() invoked for StaticMember"),
            Type::VTable(_) => panic!("type_size() invoked for VTable"),
            Type::BaseClass(_) => panic!("type_size() invoked for BaseClass"),
        }
    }

    fn on_complete(&mut self, pdb: &ParsedPdb) {
        match self {
            Type::Class(class) => class.on_complete(pdb),
            Type::Union(union) => union.on_complete(pdb),
            Type::Array(a) => a.on_complete(pdb),
            _ => {}
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypeProperties {
    pub packed: bool,
    pub constructors: bool,
    pub overlapped_operators: bool,
    pub is_nested_type: bool,
    pub contains_nested_types: bool,
    pub overload_assignment: bool,
    pub overload_coasting: bool,
    pub forward_reference: bool,
    pub scoped_definition: bool,
    pub has_unique_name: bool,
    pub sealed: bool,
    pub hfa: u8,
    pub intristic_type: bool,
    pub mocom: u8,
}

impl From<pdb::TypeProperties> for TypeProperties {
    fn from(props: pdb::TypeProperties) -> Self {
        TypeProperties {
            packed: props.packed(),
            constructors: props.constructors(),
            overlapped_operators: props.overloaded_operators(),
            is_nested_type: props.is_nested_type(),
            contains_nested_types: props.contains_nested_types(),
            overload_assignment: props.overloaded_assignment(),
            overload_coasting: props.overloaded_casting(),
            forward_reference: props.forward_reference(),
            scoped_definition: props.scoped_definition(),
            has_unique_name: props.has_unique_name(),
            sealed: props.sealed(),
            hfa: props.hfa(),
            intristic_type: props.intrinsic_type(),
            mocom: props.mocom(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Class {
    pub name: String,
    pub unique_name: Option<String>,
    pub kind: ClassKind,
    pub properties: TypeProperties,
    pub derived_from: Option<TypeRef>,
    pub fields: Vec<TypeRef>,
    pub size: usize,
}

impl Typed for Class {
    fn type_size(&self, pdb: &ParsedPdb) -> usize {
        if self.properties.forward_reference {
            // Find the implementation
            for (_key, value) in &pdb.types {
                if let Some(borrow) = value.as_ref().try_borrow().ok() {
                    if let Type::Class(class) = &*borrow {
                        if !class.properties.forward_reference
                            && class.unique_name == self.unique_name
                        {
                            return class.type_size(pdb);
                        }
                    }
                }
            }
            println!("could not get forward reference for {}", self.name);
        }

        self.size
    }
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

        let fields: Vec<TypeRef> = fields
            .map(|type_index| {
                // TODO: perhaps change FieldList to Rc<Vec<TypeRef>?
                if let Type::FieldList(fields) =
                    &*crate::parse::handle_type(type_index, output_pdb, type_finder)
                        .expect("failed to resolve dependent type")
                        .as_ref()
                        .borrow()
                {
                    fields.0.clone()
                } else {
                    panic!("got an unexpected type when FieldList was expected")
                }
            })
            .unwrap_or_default();

        let derived_from = derived_from.map(|type_index| {
            crate::parse::handle_type(type_index, output_pdb, type_finder)
                .expect("failed to resolve dependent type")
        });

        let unique_name = unique_name.map(|s| s.to_string().into_owned());

        Class {
            name: name.to_string().into_owned(),
            unique_name,
            kind: kind.into(),
            properties: properties.into(),
            derived_from,
            fields,
            size: size as usize,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct BaseClass {
    kind: ClassKind,
    base_class: TypeRef,
    offset: usize,
}

type FromBaseClass<'a, 'b> = (
    &'b pdb::BaseClassType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromBaseClass<'_, '_>> for BaseClass {
    fn from(info: FromBaseClass<'_, '_>) -> Self {
        let (class, type_finder, output_pdb) = info;

        let pdb::BaseClassType {
            kind,
            attributes,
            base_class,
            offset,
        } = *class;

        let base_class = crate::parse::handle_type(base_class, output_pdb, type_finder)
            .expect("failed to resolve dependent type");

        BaseClass {
            kind: kind.into(),
            base_class,
            offset: offset as usize,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VirtualBaseClass {
    direct: bool,
    base_class: TypeRef,
    base_pointer: TypeRef,
    base_pointer_offset: usize,
    virtual_base_offset: usize,
}

type FromVirtualBaseClass<'a, 'b> = (
    &'b pdb::VirtualBaseClassType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromVirtualBaseClass<'_, '_>> for VirtualBaseClass {
    fn from(info: FromVirtualBaseClass<'_, '_>) -> Self {
        let (class, type_finder, output_pdb) = info;

        let pdb::VirtualBaseClassType {
            direct,
            attributes,
            base_class,
            base_pointer,
            base_pointer_offset,
            virtual_base_offset,
        } = *class;

        let base_class = crate::parse::handle_type(base_class, output_pdb, type_finder)
            .expect("failed to resolve underlying type");
        let base_pointer = crate::parse::handle_type(base_pointer, output_pdb, type_finder)
            .expect("failed to resolve underlying type");

        VirtualBaseClass {
            direct,
            base_class,
            base_pointer,
            base_pointer_offset: base_pointer_offset as usize,
            virtual_base_offset: virtual_base_offset as usize,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Union {
    pub name: String,
    pub unique_name: Option<String>,
    pub properties: TypeProperties,
    pub size: usize,
    pub count: usize,
    pub fields: Vec<TypeRef>,
}

impl Typed for Union {
    fn type_size(&self, pdb: &ParsedPdb) -> usize {
        if self.properties.forward_reference {
            // Find the implementation
            for (_key, value) in &pdb.types {
                if let Some(value) = value.as_ref().try_borrow().ok() {
                    if let Type::Union(union) = &*value {
                        if !union.properties.forward_reference && union.unique_name == self.unique_name
                        {
                            return union.type_size(pdb);
                        }
                    }
                }
            }

            println!("could not get forward reference for {}", self.name);
        }
        self.size
    }
}
type FromUnion<'a, 'b> = (
    &'b pdb::UnionType<'a>,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);
impl From<FromUnion<'_, '_>> for Union {
    fn from(data: FromUnion<'_, '_>) -> Self {
        let (union, type_finder, output_pdb) = data;
        let pdb::UnionType {
            count,
            properties,
            size,
            fields,
            name,
            unique_name,
        } = union;

        let fields = crate::parse::handle_type(*fields, output_pdb, type_finder)
            .expect("failed to resolve dependent type");

        // TODO: perhaps change FieldList to Rc<Vec<TypeRef>?
        let fields = if *count > 0 {
            if let Type::FieldList(fields) = &*fields.as_ref().borrow() {
                fields.0.clone()
            } else {
                panic!(
                "got an unexpected type when FieldList was expected. union: {:#?}\n fields: {:#?}",
                union, fields
            );
            }
        } else {
            vec![]
        };

        let mut union = Union {
            name: name.to_string().into_owned(),
            unique_name: unique_name.map(|s| s.to_string().into_owned()),
            properties: (*properties).into(),
            size: *size as usize,
            count: *count as usize,
            fields,
        };

        union
    }
}

type FromBitfield<'a, 'b> = (
    &'b pdb::BitfieldType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);
#[derive(Debug, Serialize, Deserialize)]
pub struct Bitfield {
    underlying_type: TypeRef,
    len: usize,
    position: usize,
}
impl From<FromBitfield<'_, '_>> for Bitfield {
    fn from(data: FromBitfield<'_, '_>) -> Self {
        let (bitfield, type_finder, output_pdb) = data;
        let pdb::BitfieldType {
            underlying_type,
            length,
            position,
        } = *bitfield;

        let underlying_type = crate::parse::handle_type(underlying_type, output_pdb, type_finder)
            .expect("failed to parse dependent type");

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
    underlying_type: TypeRef,
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
    underlying_type: Option<TypeRef>,
    attributes: PointerAttributes,
}

type FromPointer<'a, 'b> = (
    &'b pdb::PointerType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);
impl From<FromPointer<'_, '_>> for Pointer {
    fn from(data: FromPointer<'_, '_>) -> Self {
        let (pointer, type_finder, output_pdb) = data;
        let pdb::PointerType {
            underlying_type,
            attributes,
            containing_class,
        } = *pointer;

        let underlying_type =
            crate::parse::handle_type(underlying_type, output_pdb, type_finder).ok();

        Pointer {
            underlying_type,
            attributes: attributes.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PointerKind {
    Near16,
    Far16,
    Huge16,
    BaseSeg,
    BaseVal,
    BaseSegVal,
    BaseAddr,
    BaseSegAddr,
    BaseType,
    BaseSelf,
    Near32,
    Far32,
    Ptr64,
}

impl From<pdb::PointerKind> for PointerKind {
    fn from(kind: pdb::PointerKind) -> Self {
        match kind {
            pdb::PointerKind::Near16 => PointerKind::Near16,
            pdb::PointerKind::Far16 => PointerKind::Far16,
            pdb::PointerKind::Huge16 => PointerKind::Huge16,
            pdb::PointerKind::BaseSeg => PointerKind::BaseSeg,
            pdb::PointerKind::BaseVal => PointerKind::BaseVal,
            pdb::PointerKind::BaseSegVal => PointerKind::BaseSegVal,
            pdb::PointerKind::BaseAddr => PointerKind::BaseAddr,
            pdb::PointerKind::BaseSegAddr => PointerKind::BaseSegAddr,
            pdb::PointerKind::BaseType => PointerKind::BaseType,
            pdb::PointerKind::BaseSelf => PointerKind::BaseSelf,
            pdb::PointerKind::Near32 => PointerKind::Near32,
            pdb::PointerKind::Far32 => PointerKind::Far32,
            pdb::PointerKind::Ptr64 => PointerKind::Ptr64,
        }
    }
}

impl Typed for PointerKind {
    fn type_size(&self, _pdb: &ParsedPdb) -> usize {
        match self {
            PointerKind::Near16 | PointerKind::Far16 | PointerKind::Huge16 => 2,
            PointerKind::Near32 | PointerKind::Far32 => 4,
            PointerKind::Ptr64 => 8,
            other => panic!("type_size() not implemented for pointer type: {:?}", other),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PointerAttributes {
    kind: PointerKind,
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
            kind: attr.pointer_kind().into(),
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
pub struct Primitive {
    pub kind: PrimitiveKind,
    pub indirection: Option<Indirection>,
}

impl From<&pdb::PrimitiveType> for Primitive {
    fn from(typ: &pdb::PrimitiveType) -> Self {
        let pdb::PrimitiveType { kind, indirection } = typ;

        Primitive {
            kind: kind.into(),
            indirection: indirection.map(|i| i.into()),
        }
    }
}

impl Typed for Primitive {
    fn type_size(&self, pdb: &ParsedPdb) -> usize {
        if let Some(indirection) = self.indirection.as_ref() {
            return indirection.type_size(pdb);
        }

        return self.kind.type_size(pdb);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Indirection {
    Near16,
    Far16,
    Huge16,
    Near32,
    Far32,
    Near64,
    Near128,
}

impl From<pdb::Indirection> for Indirection {
    fn from(kind: pdb::Indirection) -> Self {
        match kind {
            pdb::Indirection::Near16 => Indirection::Near16,
            pdb::Indirection::Far16 => Indirection::Far16,
            pdb::Indirection::Huge16 => Indirection::Huge16,
            pdb::Indirection::Near32 => Indirection::Near32,
            pdb::Indirection::Far32 => Indirection::Far32,
            pdb::Indirection::Near64 => Indirection::Near64,
            pdb::Indirection::Near128 => Indirection::Near128,
        }
    }
}

impl Typed for Indirection {
    fn type_size(&self, _pdb: &ParsedPdb) -> usize {
        match self {
            Indirection::Near16 | Indirection::Far16 | Indirection::Huge16 => 2,
            Indirection::Near32 | Indirection::Far32 => 4,
            Indirection::Near64 => 8,
            Indirection::Near128 => 8,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PrimitiveKind {
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

impl From<&pdb::PrimitiveKind> for PrimitiveKind {
    fn from(kind: &pdb::PrimitiveKind) -> Self {
        match *kind {
            pdb::PrimitiveKind::NoType => PrimitiveKind::NoType,
            pdb::PrimitiveKind::Void => PrimitiveKind::Void,
            pdb::PrimitiveKind::Char => PrimitiveKind::Char,
            pdb::PrimitiveKind::UChar => PrimitiveKind::UChar,
            pdb::PrimitiveKind::RChar => PrimitiveKind::RChar,
            pdb::PrimitiveKind::WChar => PrimitiveKind::WChar,
            pdb::PrimitiveKind::RChar16 => PrimitiveKind::RChar16,
            pdb::PrimitiveKind::RChar32 => PrimitiveKind::RChar32,
            pdb::PrimitiveKind::I8 => PrimitiveKind::I8,
            pdb::PrimitiveKind::U8 => PrimitiveKind::U8,
            pdb::PrimitiveKind::Short => PrimitiveKind::Short,
            pdb::PrimitiveKind::UShort => PrimitiveKind::UShort,
            pdb::PrimitiveKind::I16 => PrimitiveKind::I16,
            pdb::PrimitiveKind::U16 => PrimitiveKind::U16,
            pdb::PrimitiveKind::Long => PrimitiveKind::Long,
            pdb::PrimitiveKind::ULong => PrimitiveKind::ULong,
            pdb::PrimitiveKind::I32 => PrimitiveKind::I32,
            pdb::PrimitiveKind::U32 => PrimitiveKind::U32,
            pdb::PrimitiveKind::Quad => PrimitiveKind::Quad,
            pdb::PrimitiveKind::UQuad => PrimitiveKind::UQuad,
            pdb::PrimitiveKind::I64 => PrimitiveKind::I64,
            pdb::PrimitiveKind::U64 => PrimitiveKind::U64,
            pdb::PrimitiveKind::Octa => PrimitiveKind::Octa,
            pdb::PrimitiveKind::UOcta => PrimitiveKind::UOcta,
            pdb::PrimitiveKind::I128 => PrimitiveKind::I128,
            pdb::PrimitiveKind::U128 => PrimitiveKind::U128,
            pdb::PrimitiveKind::F16 => PrimitiveKind::F16,
            pdb::PrimitiveKind::F32 => PrimitiveKind::F32,
            pdb::PrimitiveKind::F32PP => PrimitiveKind::F32PP,
            pdb::PrimitiveKind::F48 => PrimitiveKind::F48,
            pdb::PrimitiveKind::F64 => PrimitiveKind::F64,
            pdb::PrimitiveKind::F80 => PrimitiveKind::F80,
            pdb::PrimitiveKind::F128 => PrimitiveKind::F128,
            pdb::PrimitiveKind::Complex32 => PrimitiveKind::Complex32,
            pdb::PrimitiveKind::Complex64 => PrimitiveKind::Complex64,
            pdb::PrimitiveKind::Complex80 => PrimitiveKind::Complex80,
            pdb::PrimitiveKind::Complex128 => PrimitiveKind::Complex128,
            pdb::PrimitiveKind::Bool8 => PrimitiveKind::Bool8,
            pdb::PrimitiveKind::Bool16 => PrimitiveKind::Bool16,
            pdb::PrimitiveKind::Bool32 => PrimitiveKind::Bool32,
            pdb::PrimitiveKind::Bool64 => PrimitiveKind::Bool64,
            pdb::PrimitiveKind::HRESULT => PrimitiveKind::HRESULT,
        }
    }
}

impl Typed for PrimitiveKind {
    fn type_size(&self, _pdb: &ParsedPdb) -> usize {
        match self {
            PrimitiveKind::NoType | PrimitiveKind::Void => 0,

            PrimitiveKind::Char
            | PrimitiveKind::UChar
            | PrimitiveKind::RChar
            | PrimitiveKind::I8
            | PrimitiveKind::U8
            | PrimitiveKind::Bool8 => 1,

            PrimitiveKind::RChar16
            | PrimitiveKind::WChar
            | PrimitiveKind::Short
            | PrimitiveKind::UShort
            | PrimitiveKind::I16
            | PrimitiveKind::U16
            | PrimitiveKind::F16
            | PrimitiveKind::Bool16 => 2,

            PrimitiveKind::RChar32
            | PrimitiveKind::Long
            | PrimitiveKind::ULong
            | PrimitiveKind::I32
            | PrimitiveKind::U32
            | PrimitiveKind::F32
            | PrimitiveKind::F32PP
            | PrimitiveKind::Bool32
            | PrimitiveKind::HRESULT => 4,

            PrimitiveKind::Quad
            | PrimitiveKind::UQuad
            | PrimitiveKind::I64
            | PrimitiveKind::U64
            | PrimitiveKind::F64
            | PrimitiveKind::Bool64 => 8,
            PrimitiveKind::Octa
            | PrimitiveKind::UOcta
            | PrimitiveKind::I128
            | PrimitiveKind::U128 => 16,
            _ => panic!("type size not handled for type: {:?}", self),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Array {
    element_type: TypeRef,
    indexing_type: TypeRef,
    stride: Option<u32>,
    size: usize,
    dimensions_bytes: Vec<usize>,
    dimensions_elements: Vec<usize>,
}

impl Typed for Array {
    fn type_size(&self, pdb: &ParsedPdb) -> usize {
        self.size
    }

    fn on_complete(&mut self, pdb: &ParsedPdb) {
        self.dimensions_elements.clear();

        if self.size == 0 {
            self.dimensions_elements.push(0);
            return;
        }

        let mut running_size = self.element_type.as_ref().borrow().type_size(pdb);

        for byte_size in &self.dimensions_bytes {
            let size = *byte_size / running_size;
            self.dimensions_elements.push(size);

            running_size = size;
        }
    }
}

type FromArray<'a, 'b> = (
    &'b pdb::ArrayType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromArray<'_, '_>> for Array {
    fn from(data: FromArray<'_, '_>) -> Self {
        let (array, type_finder, output_pdb) = data;

        let pdb::ArrayType {
            element_type,
            indexing_type,
            stride,
            dimensions,
        } = array;

        let element_type = crate::parse::handle_type(*element_type, output_pdb, type_finder)
            .expect("failed to parse dependent type");

        let indexing_type = crate::parse::handle_type(*indexing_type, output_pdb, type_finder)
            .expect("failed to parse dependent type");
        let size = *dimensions.last().unwrap() as usize;

        Array {
            element_type,
            indexing_type,
            stride: *stride,
            size,
            dimensions_bytes: dimensions.iter().map(|b| *b as usize).collect(),
            dimensions_elements: Vec::with_capacity(dimensions.len()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldList(Vec<TypeRef>);

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

        let mut result_fields: Vec<TypeRef> = fields
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
            let field = field.as_ref().borrow();
            if let Type::FieldList(fields) = &*field {
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
pub struct ArgumentList(Vec<TypeRef>);

type FromArgumentList<'a, 'b> = (
    &'b pdb::ArgumentList,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromArgumentList<'_, '_>> for ArgumentList {
    fn from(data: FromArgumentList<'_, '_>) -> Self {
        let (arguments, type_finder, output_pdb) = data;

        let pdb::ArgumentList { arguments } = arguments;

        let arguments: Vec<TypeRef> = arguments
            .iter()
            .map(|typ| {
                crate::parse::handle_type(*typ, output_pdb, type_finder)
                    .ok()
                    .unwrap_or_else(|| panic!("failed to parse dependent type"))
            })
            .collect();

        ArgumentList(arguments)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Modifier {
    underlying_type: TypeRef,
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
    underlying_type: TypeRef,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Procedure {
    return_type: Option<TypeRef>,
    argument_list: Vec<TypeRef>,
}

type FromProcedure<'a, 'b> = (
    &'b pdb::ProcedureType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromProcedure<'_, '_>> for Procedure {
    fn from(data: FromProcedure<'_, '_>) -> Self {
        let (proc, type_finder, output_pdb) = data;

        let pdb::ProcedureType {
            return_type,
            attributes,
            parameter_count,
            argument_list,
        } = *proc;

        let return_type = return_type.map(|return_type| {
            crate::parse::handle_type(return_type, output_pdb, type_finder)
                .expect("failed to parse dependent type")
        });

        let arguments: Vec<TypeRef>;
        let field = crate::parse::handle_type(argument_list, output_pdb, type_finder)
            .expect("failed to parse dependent type");
        if let Type::ArgumentList(argument_list) = &*field.as_ref().borrow() {
            arguments = argument_list.0.clone();
        } else {
            panic!(
                "unexpected type returned while getting FieldList continuation: {:?}",
                field
            )
        }

        Procedure {
            return_type,
            argument_list: arguments,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct MemberFunction {
    return_type: TypeRef,
    class_type: TypeRef,
    this_pointer_type: Option<TypeRef>,
    argument_list: Vec<TypeRef>,
}

type FromMemberFunction<'a, 'b> = (
    &'b pdb::MemberFunctionType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromMemberFunction<'_, '_>> for MemberFunction {
    fn from(data: FromMemberFunction<'_, '_>) -> Self {
        let (member, type_finder, output_pdb) = data;

        let pdb::MemberFunctionType {
            return_type,
            class_type,
            this_pointer_type,
            attributes,
            parameter_count,
            argument_list,
            this_adjustment,
        } = *member;

        let return_type = crate::parse::handle_type(return_type, output_pdb, type_finder)
            .expect("failed to parse dependent type");

        let class_type = crate::parse::handle_type(class_type, output_pdb, type_finder)
            .expect("failed to parse dependent type");

        let this_pointer_type = this_pointer_type.map(|ptr_type| {
            crate::parse::handle_type(ptr_type, output_pdb, type_finder)
                .expect("failed to parse dependent type")
        });

        let arguments: Vec<TypeRef>;
        let field = crate::parse::handle_type(argument_list, output_pdb, type_finder)
            .expect("failed to parse dependent type");
        if let Type::ArgumentList(argument_list) = &*field.as_ref().borrow() {
            arguments = argument_list.0.clone();
        } else {
            panic!(
                "unexpected type returned while getting FieldList continuation: {:?}",
                field
            )
        }

        MemberFunction {
            return_type,
            class_type,
            this_pointer_type,
            argument_list: arguments,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodList(Vec<MethodListEntry>);

type FromMethodList<'a, 'b> = (
    &'b pdb::MethodList,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromMethodList<'_, '_>> for MethodList {
    fn from(data: FromMethodList<'_, '_>) -> Self {
        let (method_list, type_finder, output_pdb) = data;

        let pdb::MethodList { methods } = method_list;
        let converted_methods = methods
            .iter()
            .map(|method| (method, type_finder, &mut *output_pdb).into())
            .collect();

        MethodList(converted_methods)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodListEntry {
    method_type: TypeRef,
    vtable_offset: Option<usize>,
}

type FromMethodListEntry<'a, 'b> = (
    &'b pdb::MethodListEntry,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromMethodListEntry<'_, '_>> for MethodListEntry {
    fn from(data: FromMethodListEntry<'_, '_>) -> Self {
        let (method_list, type_finder, output_pdb) = data;

        let pdb::MethodListEntry {
            attributes,
            method_type,
            vtable_offset,
        } = *method_list;

        let method_type = crate::parse::handle_type(method_type, output_pdb, type_finder)
            .expect("failed to parse dependent type");

        MethodListEntry {
            method_type,
            vtable_offset: vtable_offset.map(|offset| offset as usize),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Nested {
    name: String,
    nested_type: TypeRef,
}

type FromNested<'a, 'b> = (
    &'b pdb::NestedType<'a>,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromNested<'_, '_>> for Nested {
    fn from(data: FromNested<'_, '_>) -> Self {
        let (method_list, type_finder, output_pdb) = data;

        let pdb::NestedType {
            attributes,
            nested_type,
            name,
        } = *method_list;

        let nested_type = crate::parse::handle_type(nested_type, output_pdb, type_finder)
            .expect("failed to parse dependent type");

        Nested {
            name: name.to_string().into_owned(),
            nested_type,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OverloadedMethod {
    name: String,
    method_list: TypeRef,
}

type FromOverloadedMethod<'a, 'b> = (
    &'b pdb::OverloadedMethodType<'a>,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromOverloadedMethod<'_, '_>> for OverloadedMethod {
    fn from(data: FromOverloadedMethod<'_, '_>) -> Self {
        let (method_list, type_finder, output_pdb) = data;

        let pdb::OverloadedMethodType {
            count,
            method_list,
            name,
        } = method_list;

        let method_list = crate::parse::handle_type(*method_list, output_pdb, type_finder)
            .expect("failed to parse dependent type");

        OverloadedMethod {
            name: name.to_string().into_owned(),
            method_list,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Method {
    name: String,
    method_type: TypeRef,
    vtable_offset: Option<usize>,
}

type FromMethod<'a, 'b> = (
    &'b pdb::MethodType<'a>,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromMethod<'_, '_>> for Method {
    fn from(data: FromMethod<'_, '_>) -> Self {
        let (method_list, type_finder, output_pdb) = data;

        let pdb::MethodType {
            attributes,
            method_type,
            vtable_offset,
            name,
        } = method_list;

        let method_type = crate::parse::handle_type(*method_type, output_pdb, type_finder)
            .expect("failed to parse dependent type");

        Method {
            name: name.to_string().into_owned(),
            method_type,
            vtable_offset: vtable_offset.map(|offset| offset as usize),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StaticMember {
    name: String,
    field_type: TypeRef,
}

type FromStaticMember<'a, 'b> = (
    &'b pdb::StaticMemberType<'a>,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromStaticMember<'_, '_>> for StaticMember {
    fn from(data: FromStaticMember<'_, '_>) -> Self {
        let (member, type_finder, output_pdb) = data;

        let pdb::StaticMemberType {
            attributes,
            field_type,
            name,
        } = member;

        let field_type = crate::parse::handle_type(*field_type, output_pdb, type_finder)
            .expect("failed to parse dependent type");

        StaticMember {
            name: name.to_string().into_owned(),
            field_type,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VTable(TypeRef);
type FromVirtualFunctionTablePointer<'a, 'b> = (
    &'b pdb::VirtualFunctionTablePointerType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl From<FromVirtualFunctionTablePointer<'_, '_>> for VTable {
    fn from(data: FromVirtualFunctionTablePointer<'_, '_>) -> Self {
        let (member, type_finder, output_pdb) = data;

        let pdb::VirtualFunctionTablePointerType { table } = *member;

        let vtable_type = crate::parse::handle_type(table, output_pdb, type_finder)
            .expect("failed to parse dependent type");

        VTable(vtable_type)
    }
}
