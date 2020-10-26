use log::warn;
use pdb::FallibleIterator;
use serde::Serialize;
use std::borrow::Cow;
use std::convert::{From, TryFrom};
use std::path::PathBuf;
use std::rc::Rc;

/// Represents a PDB that has been fully parsed
#[derive(Debug, Serialize)]
pub struct ParsedPdb {
    pub path: PathBuf,
    pub assembly_info: AssemblyInfo,
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
            assembly_info: AssemblyInfo::default(),
            public_symbols: vec![],
            types: vec![],
            procedures: vec![],
            global_data: vec![],
            debug_modules: vec![],
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct AssemblyInfo {
    pub build_info: Option<BuildInfo>,
    pub compiler_info: Option<CompilerInfo>,
}

#[derive(Debug, Serialize)]
pub struct BuildInfo {
    arguments: Vec<String>,
}

impl TryFrom<(&pdb::BuildInfoSymbol, Option<&pdb::IdFinder<'_>>)> for BuildInfo {
    type Error = crate::error::ParsingError;

    fn try_from(
        info: (&pdb::BuildInfoSymbol, Option<&pdb::IdFinder<'_>>),
    ) -> Result<Self, Self::Error> {
        let (symbol, finder) = info;
        if finder.is_none() {
            return Err(crate::error::ParsingError::MissingDependency("IdFinder"));
        }

        let finder = finder.unwrap();

        let build_info = finder
            .find(symbol.id)?
            .parse()
            .expect("failed to parse build info");
        match build_info {
            pdb::IdData::BuildInfo(build_info_id) => {
                let argument_ids: Vec<_> = build_info_id
                    .arguments
                    .iter()
                    .map(|id| finder.find(*id).expect("failed to parse ID"))
                    .collect();

                panic!("{:?}", argument_ids);
            }
            _ => unreachable!(),
        };

        Err(crate::error::ParsingError::Unsupported("BuildInfo"))
    }
}

#[derive(Debug, Serialize)]
pub struct CompilerInfo {
    // TODO: cpu_type, flags, language
    pub language: String,
    pub flags: CompileFlags,
    pub cpu_type: String,
    pub frontend_version: CompilerVersion,
    pub backend_version: CompilerVersion,
    pub version_string: String,
}

impl From<pdb::CompileFlagsSymbol<'_>> for CompilerInfo {
    fn from(flags: pdb::CompileFlagsSymbol<'_>) -> Self {
        let pdb::CompileFlagsSymbol {
            language,
            flags,
            cpu_type,
            frontend_version,
            backend_version,
            version_string,
        } = flags;

        CompilerInfo {
            language: language.to_string(),
            flags: flags.into(),
            cpu_type: cpu_type.to_string(),
            frontend_version: frontend_version.into(),
            backend_version: backend_version.into(),
            version_string: version_string.to_string().into_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CompileFlags {
    /// Compiled for edit and continue.
    pub edit_and_continue: bool,
    /// Compiled without debugging info.
    pub no_debug_info: bool,
    /// Compiled with `LTCG`.
    pub link_time_codegen: bool,
    /// Compiled with `/bzalign`.
    pub no_data_align: bool,
    /// Managed code or data is present.
    pub managed: bool,
    /// Compiled with `/GS`.
    pub security_checks: bool,
    /// Compiled with `/hotpatch`.
    pub hot_patch: bool,
    /// Compiled with `CvtCIL`.
    pub cvtcil: bool,
    /// This is a MSIL .NET Module.
    pub msil_module: bool,
    /// Compiled with `/sdl`.
    pub sdl: bool,
    /// Compiled with `/ltcg:pgo` or `pgo:`.
    pub pgo: bool,
    /// This is a .exp module.
    pub exp_module: bool,
}

impl From<pdb::CompileFlags> for CompileFlags {
    fn from(flags: pdb::CompileFlags) -> Self {
        let pdb::CompileFlags {
            edit_and_continue,
            no_debug_info,
            link_time_codegen,
            no_data_align,
            managed,
            security_checks,
            hot_patch,
            cvtcil,
            msil_module,
            sdl,
            pgo,
            exp_module,
        } = flags;

        CompileFlags {
            edit_and_continue,
            no_debug_info,
            link_time_codegen,
            no_data_align,
            managed,
            security_checks,
            hot_patch,
            cvtcil,
            msil_module,
            sdl,
            pgo,
            exp_module,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CompilerVersion {
    pub major: u16,
    pub minor: u16,
    pub build: u16,
    pub qfe: Option<u16>,
}

impl From<pdb::CompilerVersion> for CompilerVersion {
    fn from(version: pdb::CompilerVersion) -> Self {
        let pdb::CompilerVersion {
            major,
            minor,
            build,
            qfe,
        } = version;

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

impl
    From<(
        &pdb::Module<'_>,
        Option<&pdb::ModuleInfo<'_>>,
        Option<&pdb::StringTable<'_>>,
    )> for DebugModule
{
    fn from(
        data: (
            &pdb::Module<'_>,
            Option<&pdb::ModuleInfo<'_>>,
            Option<&pdb::StringTable<'_>>,
        ),
    ) -> Self {
        let (module, info, string_table) = data;

        let source_files: Option<Vec<FileInfo>> = string_table
            .and_then(|string_table| {
                info.and_then(|info| {
                    info.line_program().ok().map(|prog| {
                        prog.files()
                            .map(|f| {
                                let file_name = f
                                    .name
                                    .to_string_lossy(string_table)
                                    .expect("failed to convert string")
                                    .to_string();

                                Ok(FileInfo {
                                    name: file_name,
                                    checksum: f.checksum.into(),
                                })
                            })
                            .collect()
                            .ok()
                    })
                })
            })
            .flatten();

        DebugModule {
            name: module.module_name().to_string(),
            object_file_name: module.object_file_name().to_string(),
            source_files,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PublicSymbol {
    pub name: String,
    pub is_code: bool,
    pub is_function: bool,
    pub is_managed: bool,
    pub is_msil: bool,
    pub offset: Option<usize>,
}

impl From<(pdb::PublicSymbol<'_>, usize, Option<&pdb::AddressMap<'_>>)> for PublicSymbol {
    fn from(data: (pdb::PublicSymbol<'_>, usize, Option<&pdb::AddressMap<'_>>)) -> Self {
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

        let offset = address_map.and_then(|address_map| {
            offset
                .to_rva(address_map)
                .map(|rva| u32::from(rva) as usize + base_address)
        });

        PublicSymbol {
            name: name.to_string().to_string(),
            is_code: code,
            is_function: function,
            is_managed: managed,
            is_msil: msil,
            offset,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Data {
    name: String,

    typ: Rc<Type>,

    offset: usize,
}

// pub enum Type {
//     Class(),
// }

#[derive(Debug, Serialize)]
pub struct Type {
    name: String,
    fields: Vec<(String, Type)>,

    /// length of this field in BITS
    len: usize,
}

#[derive(Debug, Serialize)]
pub struct Procedure {
    pub name: String,

    pub signature: Option<String>,

    pub offset: Option<usize>,
    pub len: usize,

    pub is_global: bool,
    pub is_dpc: bool,
    /// length of this procedure in BYTES
    pub prologue_end: usize,
    pub epilogue_start: usize,
}

impl
    From<(
        pdb::ProcedureSymbol<'_>,
        usize,
        Option<&pdb::AddressMap<'_>>,
        &pdb::ItemFinder<'_, pdb::TypeIndex>,
    )> for Procedure
{
    fn from(
        data: (
            pdb::ProcedureSymbol<'_>,
            usize,
            Option<&pdb::AddressMap<'_>>,
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

        let offset = address_map.and_then(|address_map| {
            offset
                .to_rva(address_map)
                .map(|rva| u32::from(rva) as usize + base_address)
        });

        let signature = type_finder.find(type_index).ok().map(|type_info| {
            format!(
                "{:?}",
                type_info.parse().expect("failed to parse type info")
            )
        });

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
