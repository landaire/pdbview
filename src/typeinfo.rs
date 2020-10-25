use log::warn;
use pdb::FallibleIterator;
use serde::Serialize;
use std::borrow::Cow;
use std::convert::From;
use std::path::PathBuf;
use std::rc::Rc;

/// Represents a PDB that has been fully parsed
#[derive(Debug, Serialize)]
pub struct ParsedPdb {
    pub path: PathBuf,
    pub assembly_info: Option<AssemblyInfo>,
    pub public_symbols: Vec<PublicSymbol>,
    pub types: Vec<Rc<Type>>,
    pub procedures: Vec<Procedure>,
    pub global_data: Vec<Data>,
    pub debug_modules: Vec<DebugModule>,
}

impl ParsedPdb {
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
pub struct DebugModule {
    name: String,
    object_file_name: String,
    source_files: Option<Vec<FileInfo>>,
}

#[derive(Debug, Serialize)]
enum Checksum {
    None,
    Md5(Vec<u8>),
    Sha1(Vec<u8>),
    Sha256(Vec<u8>),
}

impl From<pdb::FileChecksum<'_>> for Checksum {
    fn from(checksum: pdb::FileChecksum<'_>) -> Self {
        match checksum {
            pdb::FileChecksum::None => Checksum::None,
            pdb::FileChecksum::Md5(data) => Checksum::Md5(data.to_vec()),
            pdb::FileChecksum::Sha1(data) => Checksum::Sha1(data.to_vec()),
            pdb::FileChecksum::Sha256(data) => Checksum::Sha256(data.to_vec()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FileInfo {
    name: String,
    checksum: Checksum,
}

impl From<(&pdb::Module<'_>, Option<&pdb::ModuleInfo<'_>>, &pdb::StringTable<'_>)> for DebugModule {
    fn from(data: (&pdb::Module<'_>, Option<&pdb::ModuleInfo<'_>>, &pdb::StringTable<'_>)) -> Self {
        let (module, info, string_table) = data;

        let source_files: Option<Vec<FileInfo>> = info.map(|info| {
            info.line_program()
                .ok()
                .and_then(|prog| {
                    prog.files().map(|f| {
                        let file_name = f.name.to_string_lossy(string_table).expect("failed to convert string").to_string();

                        Ok(FileInfo {
                        name: file_name,
                        checksum: f.checksum.into(),
                    })
                }).collect().ok()
                })
        }).flatten();

        DebugModule {
            name: module.module_name().to_string(),
            object_file_name: module.object_file_name().to_string(),
            source_files,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PublicSymbol {
    name: String,
    is_code: bool,
    is_function: bool,
    is_managed: bool,
    is_msil: bool,
    offset: Option<usize>,
}

impl From<(pdb::PublicSymbol<'_>, usize, &pdb::AddressMap<'_>)> for PublicSymbol {
    fn from(data: (pdb::PublicSymbol<'_>, usize, &pdb::AddressMap<'_>)) -> Self {
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
            name: name.to_string().to_string(),
            is_code: code,
            is_function: function,
            is_managed: managed,
            is_msil: msil,
            offset: offset,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Data {
    name: String,

    typ: Rc<Type>,

    offset: usize,
}

#[derive(Debug, Serialize)]
pub struct Type {
    name: String,
    fields: Vec<(String, Type)>,

    /// length of this field in BITS
    len: usize,
}

#[derive(Debug, Serialize)]
pub struct Procedure {
    name: String,

    signature: Option<String>,

    offset: Option<usize>,
    len: usize,

    is_global: bool,
    is_dpc: bool,
    /// length of this procedure in BYTES
    prologue_end: usize,
    epilogue_start: usize,
}

impl
    From<(
        pdb::ProcedureSymbol<'_>,
        usize,
        &pdb::AddressMap<'_>,
        &pdb::ItemFinder<'_, pdb::TypeIndex>,
    )> for Procedure
{
    fn from(
        data: (
            pdb::ProcedureSymbol<'_>,
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
            name: name.to_string().to_string(),
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
