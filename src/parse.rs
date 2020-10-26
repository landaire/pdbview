use crate::error::ParsingError;
use crate::typeinfo::*;
use anyhow::Result;
use log::{debug, warn};
use pdb::*;
use std::fs::File;
use std::path::Path;
use std::convert::TryInto;

pub(crate) fn parse_pdb<P: AsRef<Path>>(path: P, base_address: Option<usize>) -> Result<ParsedPdb> {
    let file = File::open(path.as_ref())?;
    let mut pdb = Box::new(PDB::open(file)?);

    let mut output_pdb = ParsedPdb::new(path.as_ref().to_owned());
    let address_map = pdb.address_map()?;
    let string_table = pdb.string_table()?;

    // Some symbols such as build information rely on IDs being known. Iterate these to
    // build the database
    let id_information = pdb.id_information();
    let id_finder = match &id_information {
        Ok(id_information) => {
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

    // Parse type information first. Some symbol info (such as function signatures) depends
    // upon type information, but not vice versa
    let type_information = pdb.type_information()?;
    let mut type_finder = type_information.finder();
    let mut iter = type_information.iter();
    while let Some(typ) = iter.next()? {
        type_finder.update(&iter);
    }

    // Parse public symbols
    let symbol_table = pdb.global_symbols()?;
    let mut symbols = symbol_table.iter();
    while let Some(symbol) = symbols.next()? {
        if let Err(e) = handle_symbol(
            symbol,
            &mut output_pdb,
            &address_map,
            &type_finder,
            id_finder.as_ref(),
            base_address,
        ) {
            warn!("Error handling symbol {:?}: {}", symbol, e);
        }
    }

    // Parse private symbols
    let debug_info = pdb.debug_information()?;
    let mut modules = debug_info.modules()?;
    while let Some(module) = modules.next()? {
        let module_info = pdb.module_info(&module)?;
        output_pdb
            .debug_modules
            .push((&module, module_info.as_ref(), &string_table).into());
        if module_info.is_none() {
            warn!("Could not get module info for debug module: {:?}", module);
            continue;
        }

        let module_info = module_info.unwrap();
        let mut symbol_iter = module_info.symbols()?;
        while let Some(symbol) = symbol_iter.next()? {
            if let Err(e) = handle_symbol(
                symbol,
                &mut output_pdb,
                &address_map,
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
    address_map: &AddressMap,
    type_finder: &ItemFinder<'_, TypeIndex>,
    id_finder: Option<&ItemFinder<'_, IdIndex>>,
    base_address: Option<usize>,
) -> Result<(), ParsingError> {
    let base_address = base_address.unwrap_or(0);
    let sym = sym.parse()?;

    match sym {
        SymbolData::Public(data) => {
            debug!("public symbol: {:?}", data);

            let converted_symbol: crate::typeinfo::PublicSymbol =
                (data, base_address, address_map).into();
            output_pdb.public_symbols.push(converted_symbol);
        }
        SymbolData::Procedure(data) => {
            debug!("procedure: {:?}", data);

            let converted_symbol: crate::typeinfo::Procedure =
                (data, base_address, address_map, type_finder).into();
            output_pdb.procedures.push(converted_symbol);
        }
        SymbolData::BuildInfo(data) => {
            debug!("build info: {:?}", data);
            let converted_symbol: crate::typeinfo::BuildInfo =
                (&data, id_finder).try_into()?;
            output_pdb.assembly_info.build_info = Some(converted_symbol);
        }
        SymbolData::CompileFlags(data) => {
            debug!("compile flags: {:?}", data);
            let sym: crate::typeinfo::CompilerInfo = data.into();
            output_pdb.assembly_info.compiler_info = Some(sym);
        }
        other => {
            warn!("Unhandled SymbolData: {:?}", other);
        }
    }

    Ok(())
}
