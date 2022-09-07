use crate::error::Error;
use crate::symbol_types::*;
use log::{debug, warn};
use pdb::{
    AddressMap, AnnotationReferenceSymbol, FallibleIterator, IdIndex, ItemFinder, Symbol,
    SymbolData, TypeData, TypeIndex, PDB,
};
use std::cell::RefCell;
use std::convert::TryInto;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;

pub mod error;
pub mod symbol_types;
pub mod type_info;

pub use crate::symbol_types::ParsedPdb;

pub fn parse_pdb<P: AsRef<Path>>(
    path: P,
    base_address: Option<usize>,
) -> Result<ParsedPdb, crate::error::Error> {
    let file = File::open(path.as_ref())?;
    //debug!("opening PDB");
    let mut pdb = PDB::open(file)?;

    let mut output_pdb = ParsedPdb::new(path.as_ref().to_owned());
    let dbi = pdb.debug_information()?;
    let pdbi = pdb.pdb_information()?;
    output_pdb.machine_type = dbi
        .machine_type()
        .ok()
        .map(|machine_type| (&machine_type).into());

    output_pdb.age = match dbi.age() {
        Some(age) => age,
        None => pdbi.age,
    };

    output_pdb.guid = pdbi.guid;
    output_pdb.timestamp = pdbi.signature;
    output_pdb.version = (&pdbi.version).into();

    //debug!("getting address map");
    let address_map = pdb.address_map().ok();
    //debug!("grabbing string table");
    let string_table = pdb.string_table().ok();

    //debug!("fetching ID information");
    // Some symbols such as build information rely on IDs being known. Iterate these to
    // build the database
    let id_information = pdb.id_information();
    let id_finder = match &id_information {
        Ok(id_information) => {
            //debug!("ID information header was valid");
            let mut id_finder = id_information.finder();
            let mut iter = id_information.iter();
            while let Some(_id) = iter.next()? {
                id_finder.update(&iter);
            }

            Some(id_finder)
        }
        Err(e) => {
            warn!("error when fetching id_information: {}. ID information and symbols depending on such will not be loaded", e);
            None
        }
    };

    //debug!("grabbing type information");
    // Parse type information first. Some symbol info (such as function signatures) depends
    // upon type information, but not vice versa
    let type_information = pdb.type_information()?;
    let mut type_finder = type_information.finder();
    let mut iter = type_information.iter();
    let mut discovered_types = vec![];
    while let Some(typ) = iter.next()? {
        type_finder.update(&iter);
        discovered_types.push(typ.index());
    }

    for typ in discovered_types.iter() {
        let _typ = match handle_type(*typ, &mut output_pdb, &type_finder) {
            Ok(typ) => typ,
            Err(Error::PdbCrateError(e @ pdb::Error::UnimplementedTypeKind(_))) => {
                //debug!("Could not parse type: {}", e);
                continue;
            }
            // TypeNotFound is commonly raised because the PDB spec is not open, so
            // some types are unknown to this crate. We can ignore these and just fail
            // any type depending on something we cannot resolve.
            Err(Error::PdbCrateError(e @ pdb::Error::TypeNotFound(_))) => {
                //debug!("{}", e);
                continue;
            }
            Err(e) => return Err(e),
        };
    }

    // Iterate through all of the parsed types once just to update any necessary info
    for typ in output_pdb.types.values() {
        use crate::type_info::Typed;

        typ.as_ref().borrow_mut().on_complete(&output_pdb);
    }

    // Iterate through all of the parsed types once just to update any necessary info
    // for typ in output_pdb.types.values() {
    //     println!("{:#?}", typ.as_ref().borrow());
    // }

    //debug!("grabbing public symbols");
    // Parse public symbols
    let symbol_table = pdb.global_symbols()?;
    let mut symbols = symbol_table.iter();
    while let Some(symbol) = symbols.next()? {
        if let Err(e) = handle_symbol(
            symbol,
            &mut output_pdb,
            address_map.as_ref(),
            &type_finder,
            id_finder.as_ref(),
            base_address,
        ) {
            //debug!("Error handling symbol {:?}: {}", symbol, e);
        }
    }

    //debug!("grabbing debug modules");
    // Parse private symbols
    let debug_info = pdb.debug_information()?;
    let mut modules = debug_info.modules()?;
    while let Some(module) = modules.next()? {
        let module_info = pdb.module_info(&module)?;
        output_pdb
            .debug_modules
            .push((&module, module_info.as_ref(), string_table.as_ref()).into());
        if module_info.is_none() {
            //warn!("Could not get module info for debug module: {:?}", module);
            continue;
        }

        //debug!("grabbing symbols for module: {}", module.module_name());
        let module_info = module_info.unwrap();
        let mut symbol_iter = module_info.symbols()?;
        while let Some(symbol) = symbol_iter.next()? {
            if let Err(e) = handle_symbol(
                symbol,
                &mut output_pdb,
                address_map.as_ref(),
                &type_finder,
                id_finder.as_ref(),
                base_address,
            ) {
                //debug!("Error handling symbol {:?}: {}", symbol, e);
            }
        }
    }

    Ok(output_pdb)
}

/// Converts a [pdb::SymbolData] object to a parsed symbol representation that
/// we can serialize and adds it to the appropriate fields on the output [ParsedPdb].
/// Errors returned from this function should not be considered fatal.
fn handle_symbol(
    sym: Symbol,
    output_pdb: &mut ParsedPdb,
    address_map: Option<&AddressMap>,
    type_finder: &ItemFinder<'_, TypeIndex>,
    id_finder: Option<&ItemFinder<'_, IdIndex>>,
    base_address: Option<usize>,
) -> Result<(), Error> {
    let base_address = base_address.unwrap_or(0);
    let sym = sym.parse()?;

    match sym {
        SymbolData::Public(data) => {
            //debug!("public symbol: {:?}", data);

            let converted_symbol: crate::symbol_types::PublicSymbol =
                (data, base_address, address_map).into();
            output_pdb.public_symbols.push(converted_symbol);
        }
        SymbolData::Procedure(data) => {
            //debug!("procedure: {:?}", data);

            let converted_symbol: crate::symbol_types::Procedure =
                (data, base_address, address_map, type_finder).into();
            output_pdb.procedures.push(converted_symbol);
        }
        SymbolData::BuildInfo(data) => {
            //debug!("build info: {:?}", data);
            let converted_symbol: crate::symbol_types::BuildInfo = (&data, id_finder).try_into()?;
            output_pdb.assembly_info.build_info = Some(converted_symbol);
        }
        SymbolData::CompileFlags(data) => {
            //debug!("compile flags: {:?}", data);
            let sym: crate::symbol_types::CompilerInfo = data.into();
            output_pdb.assembly_info.compiler_info = Some(sym);
        }
        SymbolData::AnnotationReference(annotation) => {
            //debug!("annotation reference: {:?}", annotation);

            // let sym: crate::symbol_types::AnnotationReference = annotation.try_into()?;
            // output_pdb.annotation_references.push()
        }
        SymbolData::Data(data) => {
            let sym: crate::symbol_types::Data =
                (data, base_address, address_map, &output_pdb.types).try_into()?;
            if sym.is_global {
                output_pdb.global_data.push(sym);
            }
        }
        other => {
            //warn!("Unhandled SymbolData: {:?}", other);
        }
    }

    Ok(())
}

/// Converts a [pdb::SymbolData] object to a parsed symbol representation that
/// we can serialize and adds it to the appropriate fields on the output [ParsedPdb].
/// Errors returned from this function should not be considered fatal.
pub(crate) fn handle_type(
    idx: pdb::TypeIndex,
    output_pdb: &mut ParsedPdb,
    type_finder: &ItemFinder<'_, TypeIndex>,
) -> Result<TypeRef, Error> {
    use crate::type_info::{Class, Type, Union};
    if let Some(typ) = output_pdb.types.get(&idx.0) {
        return Ok(Rc::clone(typ));
    }

    let typ = type_finder.find(idx).expect("failed to resolve type");

    let parsed_type = &typ.parse()?;
    let typ = handle_type_data(parsed_type, output_pdb, type_finder)?;

    output_pdb.types.insert(idx.0, Rc::clone(&typ));

    Ok(typ)
}

pub(crate) fn handle_type_data(
    typ: &pdb::TypeData,
    output_pdb: &mut ParsedPdb,
    type_finder: &ItemFinder<'_, TypeIndex>,
) -> Result<TypeRef, Error> {
    use crate::type_info::{Class, Type};
    let typ = match typ {
        TypeData::Class(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::Class(typ)
        }
        TypeData::Union(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::Union(typ)
        }
        TypeData::Bitfield(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::Bitfield(typ)
        }
        TypeData::Array(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::Array(typ)
        }
        TypeData::Enumerate(data) => {
            let typ = data.try_into()?;
            Type::EnumVariant(typ)
        }
        TypeData::Enumeration(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::Enumeration(typ)
        }
        TypeData::Pointer(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::Pointer(typ)
        }
        TypeData::Primitive(data) => {
            let typ = data.try_into()?;
            Type::Primitive(typ)
        }
        TypeData::FieldList(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::FieldList(typ)
        }
        TypeData::Modifier(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::Modifier(typ)
        }
        TypeData::Member(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::Member(typ)
        }
        TypeData::ArgumentList(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::ArgumentList(typ)
        }
        TypeData::Procedure(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::Procedure(typ)
        }
        TypeData::MemberFunction(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::MemberFunction(typ)
        }
        TypeData::MethodList(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::MethodList(typ)
        }
        TypeData::VirtualBaseClass(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::VirtualBaseClass(typ)
        }
        TypeData::Nested(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::Nested(typ)
        }
        TypeData::OverloadedMethod(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::OverloadedMethod(typ)
        }
        TypeData::Method(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::Method(typ)
        }
        TypeData::StaticMember(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::StaticMember(typ)
        }
        TypeData::BaseClass(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::BaseClass(typ)
        }
        TypeData::VirtualFunctionTablePointer(data) => {
            let typ = (data, type_finder, output_pdb).try_into()?;
            Type::VTable(typ)
        }
        other => {
            //warn!("Unhandled type: {:?}", other);
            panic!("type not handled: {:?}", other);
        }
    };

    Ok(Rc::new(RefCell::new(typ)))
}
