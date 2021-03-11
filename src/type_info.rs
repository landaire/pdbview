use crate::error::Error;
use crate::symbol_types::ParsedPdb;
use crate::symbol_types::TypeRef;
use serde::{Deserialize, Serialize};
use std::convert::{From, TryFrom, TryInto};
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

impl TryFrom<pdb::TypeProperties> for TypeProperties {
    type Error = Error;
    fn try_from(props: pdb::TypeProperties) -> Result<Self, Self::Error> {
        Ok(TypeProperties {
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
        })
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
            for value in pdb.types.values() {
                if let Ok(borrow) = value.as_ref().try_borrow() {
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

impl TryFrom<FromClass<'_, '_>> for Class {
    type Error = Error;
    fn try_from(info: FromClass<'_, '_>) -> Result<Self, Self::Error> {
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
                    &*crate::handle_type(type_index, output_pdb, type_finder)
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
            crate::handle_type(type_index, output_pdb, type_finder)
                .expect("failed to resolve dependent type")
        });

        let unique_name = unique_name.map(|s| s.to_string().into_owned());

        Ok(Class {
            name: name.to_string().into_owned(),
            unique_name,
            kind: kind.try_into()?,
            properties: properties.try_into()?,
            derived_from,
            fields,
            size: size as usize,
        })
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct BaseClass {
    pub kind: ClassKind,
    pub base_class: TypeRef,
    pub offset: usize,
}

type FromBaseClass<'a, 'b> = (
    &'b pdb::BaseClassType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromBaseClass<'_, '_>> for BaseClass {
    type Error = Error;
    fn try_from(info: FromBaseClass<'_, '_>) -> Result<Self, Self::Error> {
        let (class, type_finder, output_pdb) = info;

        let pdb::BaseClassType {
            kind,
            attributes,
            base_class,
            offset,
        } = *class;

        let base_class = crate::handle_type(base_class, output_pdb, type_finder)?;

        Ok(BaseClass {
            kind: kind.try_into()?,
            base_class,
            offset: offset as usize,
        })
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

impl TryFrom<FromVirtualBaseClass<'_, '_>> for VirtualBaseClass {
    type Error = Error;
    fn try_from(info: FromVirtualBaseClass<'_, '_>) -> Result<Self, Self::Error> {
        let (class, type_finder, output_pdb) = info;

        let pdb::VirtualBaseClassType {
            direct,
            attributes,
            base_class,
            base_pointer,
            base_pointer_offset,
            virtual_base_offset,
        } = *class;

        let base_class = crate::handle_type(base_class, output_pdb, type_finder)
            .expect("failed to resolve underlying type");
        let base_pointer = crate::handle_type(base_pointer, output_pdb, type_finder)
            .expect("failed to resolve underlying type");

        Ok(VirtualBaseClass {
            direct,
            base_class,
            base_pointer,
            base_pointer_offset: base_pointer_offset as usize,
            virtual_base_offset: virtual_base_offset as usize,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClassKind {
    Class,
    Struct,
    Interface,
}

impl TryFrom<pdb::ClassKind> for ClassKind {
    type Error = Error;
    fn try_from(kind: pdb::ClassKind) -> Result<Self, Self::Error> {
        Ok(match kind {
            pdb::ClassKind::Class => ClassKind::Class,
            pdb::ClassKind::Struct => ClassKind::Struct,
            pdb::ClassKind::Interface => ClassKind::Interface,
        })
    }
}

impl std::fmt::Display for ClassKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClassKind::Class => write!(f, "Class"),
            ClassKind::Struct => write!(f, "Struct"),
            ClassKind::Interface => write!(f, "Interface"),
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
                        if !union.properties.forward_reference
                            && union.unique_name == self.unique_name
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
impl TryFrom<FromUnion<'_, '_>> for Union {
    type Error = Error;
    fn try_from(data: FromUnion<'_, '_>) -> Result<Self, Self::Error> {
        let (union, type_finder, output_pdb) = data;
        let pdb::UnionType {
            count,
            properties,
            size,
            fields,
            name,
            unique_name,
        } = union;

        let fields_type = crate::handle_type(*fields, output_pdb, type_finder)?;
        let fields;
        if *count > 0 {
            let borrowed_fields = fields_type.as_ref().borrow();
            match &*borrowed_fields {
                Type::FieldList(fields_list) => {
                    fields = fields_list.0.clone();
                }
                _ => {
                    drop(borrowed_fields);
                    fields = vec![fields_type];
                }
            }
        } else {
            fields = vec![];
        }

        let mut union = Union {
            name: name.to_string().into_owned(),
            unique_name: unique_name.map(|s| s.to_string().into_owned()),
            properties: (*properties).try_into()?,
            size: *size as usize,
            count: *count as usize,
            fields,
        };

        Ok(union)
    }
}

type FromBitfield<'a, 'b> = (
    &'b pdb::BitfieldType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);
#[derive(Debug, Serialize, Deserialize)]
pub struct Bitfield {
    pub underlying_type: TypeRef,
    pub len: usize,
    pub position: usize,
}
impl TryFrom<FromBitfield<'_, '_>> for Bitfield {
    type Error = Error;
    fn try_from(data: FromBitfield<'_, '_>) -> Result<Self, Self::Error> {
        let (bitfield, type_finder, output_pdb) = data;
        let pdb::BitfieldType {
            underlying_type,
            length,
            position,
        } = *bitfield;

        let underlying_type = crate::handle_type(underlying_type, output_pdb, type_finder)?;

        Ok(Bitfield {
            underlying_type,
            len: length as usize,
            position: position as usize,
        })
    }
}

impl Typed for Bitfield {
    fn type_size(&self, pdb: &ParsedPdb) -> usize {
        panic!("calling type_size() directly on a bitfield is probably not what you want");
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Enumeration {
    pub name: String,
    pub unique_name: Option<String>,
    pub underlying_type: TypeRef,
    pub variants: Vec<EnumVariant>,
}

type FromEnumeration<'a, 'b> = (
    &'b pdb::EnumerationType<'a>,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromEnumeration<'_, '_>> for Enumeration {
    type Error = Error;
    fn try_from(data: FromEnumeration<'_, '_>) -> Result<Self, Self::Error> {
        let (e, type_finder, output_pdb) = data;

        let pdb::EnumerationType {
            count,
            properties,
            underlying_type,
            fields,
            name,
            unique_name,
        } = e;

        let underlying_type = crate::handle_type(*underlying_type, output_pdb, type_finder)?;
        // TODO: Variants

        Ok(Enumeration {
            name: name.to_string().into_owned(),
            unique_name: unique_name.map(|s| s.to_string().into_owned()),
            underlying_type,
            variants: vec![],
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnumVariant {
    name: String,
    value: VariantValue,
}

type FromEnumerate<'a, 'b> = &'b pdb::EnumerateType<'a>;

impl TryFrom<FromEnumerate<'_, '_>> for EnumVariant {
    type Error = Error;
    fn try_from(data: FromEnumerate<'_, '_>) -> Result<Self, Self::Error> {
        let e = data;

        let pdb::EnumerateType {
            attributes,
            value,
            name,
        } = e;

        Ok(Self {
            name: name.to_string().into_owned(),
            value: value.try_into()?,
        })
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

impl TryFrom<&FromVariant> for VariantValue {
    type Error = Error;
    fn try_from(data: &FromVariant) -> Result<Self, Self::Error> {
        let variant = data;

        let value = match *variant {
            pdb::Variant::U8(val) => VariantValue::U8(val),
            pdb::Variant::U16(val) => VariantValue::U16(val),
            pdb::Variant::U32(val) => VariantValue::U32(val),
            pdb::Variant::U64(val) => VariantValue::U64(val),
            pdb::Variant::I8(val) => VariantValue::I8(val),
            pdb::Variant::I16(val) => VariantValue::I16(val),
            pdb::Variant::I32(val) => VariantValue::I32(val),
            pdb::Variant::I64(val) => VariantValue::I64(val),
        };

        Ok(value)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pointer {
    pub underlying_type: Option<TypeRef>,
    pub attributes: PointerAttributes,
}

type FromPointer<'a, 'b> = (
    &'b pdb::PointerType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);
impl TryFrom<FromPointer<'_, '_>> for Pointer {
    type Error = Error;
    fn try_from(data: FromPointer<'_, '_>) -> Result<Self, Self::Error> {
        let (pointer, type_finder, output_pdb) = data;
        let pdb::PointerType {
            underlying_type,
            attributes,
            containing_class,
        } = *pointer;

        let underlying_type = crate::handle_type(underlying_type, output_pdb, type_finder).ok();

        Ok(Pointer {
            underlying_type,
            attributes: attributes.try_into()?,
        })
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

impl TryFrom<pdb::PointerKind> for PointerKind {
    type Error = Error;
    fn try_from(kind: pdb::PointerKind) -> Result<Self, Self::Error> {
        let kind = match kind {
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
        };

        Ok(kind)
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
    pub kind: PointerKind,
    pub is_volatile: bool,
    pub is_const: bool,
    pub is_unaligned: bool,
    pub is_restrict: bool,
    pub is_reference: bool,
    pub size: usize,
    pub is_mocom: bool,
}

impl TryFrom<pdb::PointerAttributes> for PointerAttributes {
    type Error = Error;
    fn try_from(attr: pdb::PointerAttributes) -> Result<Self, Self::Error> {
        let attr = PointerAttributes {
            kind: attr.pointer_kind().try_into()?,
            is_volatile: attr.is_volatile(),
            is_const: attr.is_const(),
            is_unaligned: attr.is_unaligned(),
            is_restrict: attr.is_restrict(),
            is_reference: attr.is_reference(),
            size: attr.size() as usize,
            is_mocom: attr.is_mocom(),
        };

        Ok(attr)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Primitive {
    pub kind: PrimitiveKind,
    pub indirection: Option<Indirection>,
}

impl TryFrom<&pdb::PrimitiveType> for Primitive {
    type Error = Error;
    fn try_from(typ: &pdb::PrimitiveType) -> Result<Self, Self::Error> {
        let pdb::PrimitiveType { kind, indirection } = typ;

        let prim = Primitive {
            kind: kind.try_into()?,
            indirection: indirection.map(|i| i.try_into()).transpose()?,
        };

        Ok(prim)
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

impl TryFrom<pdb::Indirection> for Indirection {
    type Error = Error;
    fn try_from(kind: pdb::Indirection) -> Result<Self, Self::Error> {
        let kind = match kind {
            pdb::Indirection::Near16 => Indirection::Near16,
            pdb::Indirection::Far16 => Indirection::Far16,
            pdb::Indirection::Huge16 => Indirection::Huge16,
            pdb::Indirection::Near32 => Indirection::Near32,
            pdb::Indirection::Far32 => Indirection::Far32,
            pdb::Indirection::Near64 => Indirection::Near64,
            pdb::Indirection::Near128 => Indirection::Near128,
        };

        Ok(kind)
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

impl TryFrom<&pdb::PrimitiveKind> for PrimitiveKind {
    type Error = Error;
    fn try_from(kind: &pdb::PrimitiveKind) -> Result<Self, Self::Error> {
        let kind = match *kind {
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
            other => return Err(Error::UnhandledType(format!("{:?}", other))),
        };

        Ok(kind)
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

impl std::fmt::Display for PrimitiveKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveKind::NoType => write!(f, "NoType"),
            PrimitiveKind::Void => write!(f, "Void"),
            PrimitiveKind::Char => write!(f, "Char"),
            PrimitiveKind::UChar => write!(f, "UChar"),
            PrimitiveKind::RChar => write!(f, "RChar"),
            PrimitiveKind::WChar => write!(f, "WChar"),
            PrimitiveKind::RChar16 => write!(f, "RChar16"),
            PrimitiveKind::RChar32 => write!(f, "RChar32"),
            PrimitiveKind::I8 => write!(f, "I8"),
            PrimitiveKind::U8 => write!(f, "U8"),
            PrimitiveKind::Short => write!(f, "Short"),
            PrimitiveKind::UShort => write!(f, "UShort"),
            PrimitiveKind::I16 => write!(f, "I16"),
            PrimitiveKind::U16 => write!(f, "U16"),
            PrimitiveKind::Long => write!(f, "Long"),
            PrimitiveKind::ULong => write!(f, "ULong"),
            PrimitiveKind::I32 => write!(f, "I32"),
            PrimitiveKind::U32 => write!(f, "U32"),
            PrimitiveKind::Quad => write!(f, "Quad"),
            PrimitiveKind::UQuad => write!(f, "UQuad"),
            PrimitiveKind::I64 => write!(f, "I64"),
            PrimitiveKind::U64 => write!(f, "U64"),
            PrimitiveKind::Octa => write!(f, "Octa"),
            PrimitiveKind::UOcta => write!(f, "UOcta"),
            PrimitiveKind::I128 => write!(f, "I128"),
            PrimitiveKind::U128 => write!(f, "U128"),
            PrimitiveKind::F16 => write!(f, "F16"),
            PrimitiveKind::F32 => write!(f, "F32"),
            PrimitiveKind::F32PP => write!(f, "F32PP"),
            PrimitiveKind::F48 => write!(f, "F48"),
            PrimitiveKind::F64 => write!(f, "F64"),
            PrimitiveKind::F80 => write!(f, "F80"),
            PrimitiveKind::F128 => write!(f, "F128"),
            PrimitiveKind::Complex32 => write!(f, "Complex32"),
            PrimitiveKind::Complex64 => write!(f, "Complex64"),
            PrimitiveKind::Complex80 => write!(f, "Complex80"),
            PrimitiveKind::Complex128 => write!(f, "Complex128"),
            PrimitiveKind::Bool8 => write!(f, "Bool8"),
            PrimitiveKind::Bool16 => write!(f, "Bool16"),
            PrimitiveKind::Bool32 => write!(f, "Bool32"),
            PrimitiveKind::Bool64 => write!(f, "Bool64"),
            PrimitiveKind::HRESULT => write!(f, "HRESULT"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Array {
    pub element_type: TypeRef,
    pub indexing_type: TypeRef,
    pub stride: Option<u32>,
    pub size: usize,
    pub dimensions_bytes: Vec<usize>,
    pub dimensions_elements: Vec<usize>,
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

impl TryFrom<FromArray<'_, '_>> for Array {
    type Error = Error;
    fn try_from(data: FromArray<'_, '_>) -> Result<Self, Self::Error> {
        let (array, type_finder, output_pdb) = data;

        let pdb::ArrayType {
            element_type,
            indexing_type,
            stride,
            dimensions,
        } = array;

        let element_type = crate::handle_type(*element_type, output_pdb, type_finder)?;
        let indexing_type = crate::handle_type(*indexing_type, output_pdb, type_finder)?;
        let size = *dimensions.last().unwrap() as usize;

        let arr = Array {
            element_type,
            indexing_type,
            stride: *stride,
            size,
            dimensions_bytes: dimensions.iter().map(|b| *b as usize).collect(),
            dimensions_elements: Vec::with_capacity(dimensions.len()),
        };

        Ok(arr)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldList(Vec<TypeRef>);

type FromFieldList<'a, 'b> = (
    &'b pdb::FieldList<'b>,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromFieldList<'_, '_>> for FieldList {
    type Error = Error;
    fn try_from(data: FromFieldList<'_, '_>) -> Result<Self, Self::Error> {
        let (fields, type_finder, output_pdb) = data;

        let pdb::FieldList {
            fields,
            continuation,
        } = fields;

        let result_fields: Result<Vec<TypeRef>, Self::Error> = fields
            .iter()
            .map(|typ| crate::handle_type_data(typ, output_pdb, type_finder))
            .collect();

        let mut result_fields = result_fields?;

        if let Some(continuation) = continuation {
            let field = crate::handle_type(*continuation, output_pdb, type_finder)?;
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

        Ok(FieldList(result_fields))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArgumentList(Vec<TypeRef>);

type FromArgumentList<'a, 'b> = (
    &'b pdb::ArgumentList,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromArgumentList<'_, '_>> for ArgumentList {
    type Error = Error;
    fn try_from(data: FromArgumentList<'_, '_>) -> Result<Self, Self::Error> {
        let (arguments, type_finder, output_pdb) = data;

        let pdb::ArgumentList { arguments } = arguments;

        let arguments: Result<Vec<TypeRef>, Self::Error> = arguments
            .iter()
            .map(|typ| crate::handle_type(*typ, output_pdb, type_finder))
            .collect();

        Ok(ArgumentList(arguments?))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Modifier {
    pub underlying_type: TypeRef,
    pub constant: bool,
    pub volatile: bool,
    pub unaligned: bool,
}

type FromModifier<'a, 'b> = (
    &'b pdb::ModifierType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromModifier<'_, '_>> for Modifier {
    type Error = Error;
    fn try_from(data: FromModifier<'_, '_>) -> Result<Self, Self::Error> {
        let (modifier, type_finder, output_pdb) = data;

        let pdb::ModifierType {
            underlying_type,
            constant,
            volatile,
            unaligned,
        } = *modifier;

        let underlying_type = crate::handle_type(underlying_type, output_pdb, type_finder)?;

        Ok(Modifier {
            underlying_type,
            constant,
            volatile,
            unaligned,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Member {
    pub name: String,
    pub underlying_type: TypeRef,
    pub offset: usize,
}

type FromMember<'a, 'b> = (
    &'b pdb::MemberType<'a>,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromMember<'_, '_>> for Member {
    type Error = Error;

    fn try_from(data: FromMember<'_, '_>) -> Result<Self, Self::Error> {
        let (member, type_finder, output_pdb) = data;

        let pdb::MemberType {
            attributes,
            field_type,
            offset,
            name,
        } = *member;

        let underlying_type = crate::handle_type(field_type, output_pdb, type_finder)?;

        Ok(Member {
            name: name.to_string().into_owned(),
            underlying_type,
            offset: offset as usize,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Procedure {
    pub return_type: Option<TypeRef>,
    pub argument_list: Vec<TypeRef>,
}

type FromProcedure<'a, 'b> = (
    &'b pdb::ProcedureType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromProcedure<'_, '_>> for Procedure {
    type Error = Error;
    fn try_from(data: FromProcedure<'_, '_>) -> Result<Self, Self::Error> {
        let (proc, type_finder, output_pdb) = data;

        let pdb::ProcedureType {
            return_type,
            attributes,
            parameter_count,
            argument_list,
        } = *proc;

        let return_type = return_type
            .map(|return_type| crate::handle_type(return_type, output_pdb, type_finder))
            .transpose()?;

        let arguments: Vec<TypeRef>;
        let field = crate::handle_type(argument_list, output_pdb, type_finder)?;
        if let Type::ArgumentList(argument_list) = &*field.as_ref().borrow() {
            arguments = argument_list.0.clone();
        } else {
            panic!(
                "unexpected type returned while getting FieldList continuation: {:?}",
                field
            )
        }

        Ok(Procedure {
            return_type,
            argument_list: arguments,
        })
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

impl TryFrom<FromMemberFunction<'_, '_>> for MemberFunction {
    type Error = Error;
    fn try_from(data: FromMemberFunction<'_, '_>) -> Result<Self, Self::Error> {
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

        let return_type = crate::handle_type(return_type, output_pdb, type_finder)?;

        let class_type = crate::handle_type(class_type, output_pdb, type_finder)?;

        let this_pointer_type = this_pointer_type
            .map(|ptr_type| crate::handle_type(ptr_type, output_pdb, type_finder))
            .transpose()?;

        let arguments: Vec<TypeRef>;
        let field = crate::handle_type(argument_list, output_pdb, type_finder)?;
        if let Type::ArgumentList(argument_list) = &*field.as_ref().borrow() {
            arguments = argument_list.0.clone();
        } else {
            panic!(
                "unexpected type returned while getting FieldList continuation: {:?}",
                field
            )
        }

        Ok(MemberFunction {
            return_type,
            class_type,
            this_pointer_type,
            argument_list: arguments,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodList(Vec<MethodListEntry>);

type FromMethodList<'a, 'b> = (
    &'b pdb::MethodList,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromMethodList<'_, '_>> for MethodList {
    type Error = Error;
    fn try_from(data: FromMethodList<'_, '_>) -> Result<Self, Self::Error> {
        let (method_list, type_finder, output_pdb) = data;

        let pdb::MethodList { methods } = method_list;
        let converted_methods: Result<Vec<MethodListEntry>, Self::Error> = methods
            .iter()
            .map(|method| (method, type_finder, &mut *output_pdb).try_into())
            .collect();

        Ok(MethodList(converted_methods?))
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

impl TryFrom<FromMethodListEntry<'_, '_>> for MethodListEntry {
    type Error = Error;
    fn try_from(data: FromMethodListEntry<'_, '_>) -> Result<Self, Self::Error> {
        let (method_list, type_finder, output_pdb) = data;

        let pdb::MethodListEntry {
            attributes,
            method_type,
            vtable_offset,
        } = *method_list;

        let method_type = crate::handle_type(method_type, output_pdb, type_finder)?;

        Ok(MethodListEntry {
            method_type,
            vtable_offset: vtable_offset.map(|offset| offset as usize),
        })
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

impl TryFrom<FromNested<'_, '_>> for Nested {
    type Error = Error;
    fn try_from(data: FromNested<'_, '_>) -> Result<Self, Self::Error> {
        let (method_list, type_finder, output_pdb) = data;

        let pdb::NestedType {
            attributes,
            nested_type,
            name,
        } = *method_list;

        let nested_type = crate::handle_type(nested_type, output_pdb, type_finder)?;

        Ok(Nested {
            name: name.to_string().into_owned(),
            nested_type,
        })
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

impl TryFrom<FromOverloadedMethod<'_, '_>> for OverloadedMethod {
    type Error = Error;
    fn try_from(data: FromOverloadedMethod<'_, '_>) -> Result<Self, Self::Error> {
        let (method_list, type_finder, output_pdb) = data;

        let pdb::OverloadedMethodType {
            count,
            method_list,
            name,
        } = method_list;

        let method_list = crate::handle_type(*method_list, output_pdb, type_finder)?;

        Ok(OverloadedMethod {
            name: name.to_string().into_owned(),
            method_list,
        })
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

impl TryFrom<FromMethod<'_, '_>> for Method {
    type Error = Error;
    fn try_from(data: FromMethod<'_, '_>) -> Result<Self, Self::Error> {
        let (method_list, type_finder, output_pdb) = data;

        let pdb::MethodType {
            attributes,
            method_type,
            vtable_offset,
            name,
        } = method_list;

        let method_type = crate::handle_type(*method_type, output_pdb, type_finder)?;

        Ok(Method {
            name: name.to_string().into_owned(),
            method_type,
            vtable_offset: vtable_offset.map(|offset| offset as usize),
        })
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

impl TryFrom<FromStaticMember<'_, '_>> for StaticMember {
    type Error = Error;
    fn try_from(data: FromStaticMember<'_, '_>) -> Result<Self, Self::Error> {
        let (member, type_finder, output_pdb) = data;

        let pdb::StaticMemberType {
            attributes,
            field_type,
            name,
        } = member;

        let field_type = crate::handle_type(*field_type, output_pdb, type_finder)
            .expect("failed to parse dependent type");

        Ok(StaticMember {
            name: name.to_string().into_owned(),
            field_type,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VTable(TypeRef);
type FromVirtualFunctionTablePointer<'a, 'b> = (
    &'b pdb::VirtualFunctionTablePointerType,
    &'b pdb::TypeFinder<'a>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromVirtualFunctionTablePointer<'_, '_>> for VTable {
    type Error = Error;
    fn try_from(data: FromVirtualFunctionTablePointer<'_, '_>) -> Result<Self, Self::Error> {
        let (member, type_finder, output_pdb) = data;

        let pdb::VirtualFunctionTablePointerType { table } = *member;

        let vtable_type = crate::handle_type(table, output_pdb, type_finder)
            .expect("failed to parse dependent type");

        Ok(VTable(vtable_type))
    }
}
