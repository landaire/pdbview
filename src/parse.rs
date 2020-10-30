use crate::error::ParsingError;
use crate::symbol_types::*;
use anyhow::Result;
use log::{debug, warn};
use pdb::*;
use std::convert::TryInto;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;

pub(crate) fn parse_pdb<P: AsRef<Path>>(path: P, base_address: Option<usize>) -> Result<ParsedPdb> {
    let file = File::open(path.as_ref())?;
    debug!("opening PDB");
    let mut pdb = Box::new(PDB::open(file)?);

    let mut output_pdb = ParsedPdb::new(path.as_ref().to_owned());
    debug!("getting address map");
    let address_map = pdb.address_map().ok();
    debug!("grabbing string table");
    let string_table = pdb.string_table().ok();

    debug!("fetching ID information");
    // Some symbols such as build information rely on IDs being known. Iterate these to
    // build the database
    let id_information = pdb.id_information();
    let id_finder = match &id_information {
        Ok(id_information) => {
            debug!("ID information header was valid");
            let mut id_finder = id_information.finder();
            let mut iter = id_information.iter();
            while let Some(id) = iter.next()? {
                id_finder.update(&iter);
            }

            Some(id_finder)
        }
        Err(e) => {
            warn!("error when fetching id_information: {}. ID information and symbols depending on such will not be loaded", e);
            None
        }
    };

    debug!("grabbing type information");
    // Parse type information first. Some symbol info (such as function signatures) depends
    // upon type information, but not vice versa
    let type_information = pdb.type_information()?;
    let mut type_finder = type_information.finder();
    let mut iter = type_information.iter();
    while let Some(typ) = iter.next()? {
        type_finder.update(&iter);
    }

    let mut iter = type_information.iter();
    while let Some(typ) = iter.next()? {
        let typ = handle_type(typ.index(), &mut output_pdb, &type_finder)?;
        println!("{:#?}", typ);
    }

    debug!("grabbing public symbols");
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
            warn!("Error handling symbol {:?}: {}", symbol, e);
        }
    }

    debug!("grabbing debug modules");
    // Parse private symbols
    let debug_info = pdb.debug_information()?;
    let mut modules = debug_info.modules()?;
    while let Some(module) = modules.next()? {
        let module_info = pdb.module_info(&module)?;
        output_pdb
            .debug_modules
            .push((&module, module_info.as_ref(), string_table.as_ref()).into());
        if module_info.is_none() {
            warn!("Could not get module info for debug module: {:?}", module);
            continue;
        }

        debug!("grabbing symbols for module: {}", module.module_name());
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
                warn!("Error handling symbol {:?}: {}", symbol, e);
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
) -> Result<(), ParsingError> {
    let base_address = base_address.unwrap_or(0);
    let sym = sym.parse()?;

    match sym {
        SymbolData::Public(data) => {
            debug!("public symbol: {:?}", data);

            let converted_symbol: crate::symbol_types::PublicSymbol =
                (data, base_address, address_map).into();
            output_pdb.public_symbols.push(converted_symbol);
        }
        SymbolData::Procedure(data) => {
            debug!("procedure: {:?}", data);

            let converted_symbol: crate::symbol_types::Procedure =
                (data, base_address, address_map, type_finder).into();
            output_pdb.procedures.push(converted_symbol);
        }
        SymbolData::BuildInfo(data) => {
            debug!("build info: {:?}", data);
            let converted_symbol: crate::symbol_types::BuildInfo = (&data, id_finder).try_into()?;
            output_pdb.assembly_info.build_info = Some(converted_symbol);
        }
        SymbolData::CompileFlags(data) => {
            debug!("compile flags: {:?}", data);
            let sym: crate::symbol_types::CompilerInfo = data.into();
            output_pdb.assembly_info.compiler_info = Some(sym);
        }
        other => {
            warn!("Unhandled SymbolData: {:?}", other);
        }
    }

    Ok(())
}

/// Converts a [pdb::SymbolData] object to a parsed symbol representation that
/// we can serialize and adds it to the appropriate fields on the output [ParsedPdb].
/// Errors returned from this function should not be considered fatal.
pub fn handle_type(
    idx: pdb::TypeIndex,
    output_pdb: &mut ParsedPdb,
    type_finder: &ItemFinder<'_, TypeIndex>,
) -> Result<Rc<crate::type_info::Type>, ParsingError> {
    if let Some(typ) = output_pdb.types.get(&idx.0) {
        return Ok(Rc::clone(typ));
    }

    let typ = type_finder.find(idx).expect("failed to resolve type");

    let typ = handle_type_data(&typ.parse()?, output_pdb, type_finder)?;
    output_pdb.types.insert(idx.0, Rc::clone(&typ));

    Ok(typ)
}

pub fn handle_type_data(
    typ: &pdb::TypeData,
    output_pdb: &mut ParsedPdb,
    type_finder: &ItemFinder<'_, TypeIndex>,
) -> Result<Rc<crate::type_info::Type>, ParsingError> {
    use crate::type_info::Type;
    let typ = match typ {
        TypeData::Class(data) => {
            let class: crate::type_info::Class = (data, type_finder, output_pdb).into();
            Rc::new(Type::Class(class))
        }
        TypeData::Union(data) => {
            let typ: crate::type_info::Union = (data, type_finder, output_pdb).into();
            Rc::new(Type::Union(typ))
        }
        TypeData::Bitfield(data) => {
            let typ: crate::type_info::Bitfield = (data, type_finder, &output_pdb.types).into();
            Rc::new(Type::Bitfield(typ))
        }
        TypeData::Array(data) => {
            let typ: crate::type_info::Array = (data, type_finder, &output_pdb.types).into();
            Rc::new(Type::Array(typ))
        }
        TypeData::Enumerate(data) => {
            let typ: crate::type_info::EnumVariant = data.into();
            Rc::new(Type::EnumVariant(typ))
        }
        TypeData::Enumeration(data) => {
            let typ: crate::type_info::Enumeration = (data, type_finder, output_pdb).into();
            Rc::new(Type::Enumeration(typ))
        }
        TypeData::Pointer(data) => {
            let typ: crate::type_info::Pointer = (data, type_finder, &output_pdb.types).into();
            Rc::new(Type::Pointer(typ))
        }
        TypeData::Primitive(data) => {
            let typ: crate::type_info::Primitive = data.into();
            Rc::new(Type::Primitive(typ))
        }
        TypeData::FieldList(data) => {
            let typ: crate::type_info::FieldList = (data, type_finder, output_pdb).into();
            Rc::new(Type::FieldList(typ))
        }
        TypeData::Modifier(data) => {
            let typ: crate::type_info::Modifier = (data, type_finder, output_pdb).into();
            Rc::new(Type::Modifier(typ))
        }
        TypeData::Member(data) => {
            let typ: crate::type_info::Member = (data, type_finder, output_pdb).into();
            Rc::new(Type::Member(typ))
        }
        other => {
            warn!("Unhandled type: {:?}", other);
            panic!("type not handled: {:?}", other);
        }
    };

    Ok(typ)
}
