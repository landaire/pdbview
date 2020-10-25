use log::warn;
use serde::Serialize;
use std::borrow::Cow;
use std::convert::From;
use std::path::PathBuf;
use std::rc::Rc;

/// Represents a PDB that has been fully parsed
#[derive(Debug, Serialize)]
pub struct ParsedPdb<'a> {
    pub path: PathBuf,
    pub assembly_info: Option<AssemblyInfo>,
    pub public_symbols: Vec<PublicSymbol<'a>>,
    pub types: Vec<Rc<Type<'a>>>,
    pub procedures: Vec<Procedure<'a>>,
    pub global_data: Vec<Data<'a>>,
    pub debug_modules: Vec<DebugModule<'a>>,
}

impl<'a> ParsedPdb<'a> {
    /// Constructs a new [ParsedPdb] with the corresponding path
    pub fn new(path: PathBuf) -> Self {
        ParsedPdb {
            path,
            assembly_info: None,
            public_symbols: vec![],
            types: vec![],
            procedures: vec![],
            global_data: vec![],
            debug_modules: vec![],
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AssemblyInfo {
    compiler_info: CompilerInfo,
}

#[derive(Debug, Serialize)]
pub struct CompilerInfo {
    // TODO: cpu_type, flags, language
    frontend_version: CompilerVersion,
    backend_version: CompilerVersion,
    version_string: String,
}

#[derive(Debug, Serialize)]
pub struct CompilerVersion {
    major: u16,
    minor: u16,
    build: u16,
    qfe: Option<u16>,
}

impl From<&pdb::CompilerVersion> for CompilerVersion {
    fn from(version: &pdb::CompilerVersion) -> Self {
        let pdb::CompilerVersion {
            major,
            minor,
            build,
            qfe,
        } = *version;

        CompilerVersion {
            major,
            minor,
            build,
            qfe,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DebugModule<'a> {
    #[serde(borrow)]
    name: Cow<'a, str>,
    #[serde(borrow)]
    object_file_name: Cow<'a, str>,
}

impl<'a> From<pdb::Module<'a>> for DebugModule<'a> {
    fn from(module: pdb::Module<'a>) -> Self {
        DebugModule {
            name: module.module_name(),
            object_file_name: module.object_file_name(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PublicSymbol<'a> {
    name: Cow<'a, str>,
    is_code: bool,
    is_function: bool,
    is_managed: bool,
    is_msil: bool,
    offset: Option<usize>,
}

impl<'a> From<(pdb::PublicSymbol<'a>, usize, &pdb::AddressMap<'_>)> for PublicSymbol<'a> {
    fn from(data: (pdb::PublicSymbol<'a>, usize, &pdb::AddressMap<'_>)) -> Self {
        let (sym, base_address, address_map) = data;

        let pdb::PublicSymbol {
            code,
            function,
            managed,
            msil,
            offset,
            name,
        } = sym;

        if offset.section == 0 {
            warn!(
                "symbol type has an invalid section index and RVA will be invalid: {:?}",
                sym
            )
        }

        let offset = offset
            .to_rva(address_map)
            .map(|rva| u32::from(rva) as usize + base_address);

        PublicSymbol {
            name: name.to_string(),
            is_code: code,
            is_function: function,
            is_managed: managed,
            is_msil: msil,
            offset: offset,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Data<'a> {
    #[serde(borrow)]
    name: Cow<'a, str>,

    #[serde(borrow)]
    typ: Rc<Type<'a>>,

    offset: usize,
}

#[derive(Debug, Serialize)]
pub struct Type<'a> {
    name: Cow<'a, str>,
    fields: Vec<(Cow<'a, str>, Type<'a>)>,

    /// length of this field in BITS
    len: usize,
}

#[derive(Debug, Serialize)]
pub struct Procedure<'a> {
    #[serde(borrow)]
    name: Cow<'a, str>,

    signature: Option<String>,

    offset: Option<usize>,
    len: usize,

    is_global: bool,
    is_dpc: bool,
    /// length of this procedure in BYTES
    prologue_end: usize,
    epilogue_start: usize,
}

impl<'a>
    From<(
        pdb::ProcedureSymbol<'a>,
        usize,
        &pdb::AddressMap<'_>,
        &pdb::ItemFinder<'_, pdb::TypeIndex>,
    )> for Procedure<'a>
{
    fn from(
        data: (
            pdb::ProcedureSymbol<'a>,
            usize,
            &pdb::AddressMap<'_>,
            &pdb::ItemFinder<'_, pdb::TypeIndex>,
        ),
    ) -> Self {
        let (sym, base_address, address_map, type_finder) = data;

        let pdb::ProcedureSymbol {
            global,
            dpc,
            parent,
            end,
            next,
            len,
            dbg_start_offset,
            dbg_end_offset,
            type_index,
            offset,
            flags,
            name,
        } = sym;

        if offset.section == 0 {
            warn!(
                "symbol type has an invalid section index and RVA will be invalid: {:?}",
                sym
            )
        }

        let offset = offset
            .to_rva(address_map)
            .map(|rva| u32::from(rva) as usize + base_address);
        let signature = type_finder
            .find(type_index)
            .ok()
            .map(|type_info| format!("{}", type_info.index()));

        Procedure {
            name: name.to_string(),
            signature,
            offset,
            len: len as usize,
            is_global: global,
            is_dpc: dpc,
            prologue_end: dbg_start_offset as usize,
            epilogue_start: dbg_end_offset as usize,
        }
    }
}
