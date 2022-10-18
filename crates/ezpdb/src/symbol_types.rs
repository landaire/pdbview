use crate::type_info::Type;
use log::warn;
use pdb::{FallibleIterator, TypeIndex};
#[cfg(feature = "serde")]
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::{From, TryFrom};
use std::path::PathBuf;
use std::rc::Rc;

pub type TypeRef = Rc<RefCell<Type>>;
pub type TypeIndexNumber = u32;
/// Represents a PDB that has been fully parsed
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct ParsedPdb {
    pub path: PathBuf,
    pub assembly_info: AssemblyInfo,
    pub public_symbols: Vec<PublicSymbol>,
    pub types: HashMap<TypeIndexNumber, TypeRef>,
    pub procedures: Vec<Procedure>,
    pub global_data: Vec<Data>,
    pub debug_modules: Vec<DebugModule>,
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    pub(crate) forward_references: Vec<Rc<Type>>,
    pub version: Version,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_uuid"))]
    pub guid: uuid::Uuid,
    pub age: u32,
    pub timestamp: u32,
    pub machine_type: Option<MachineType>,
}

impl ParsedPdb {
    /// Constructs a new [ParsedPdb] with the corresponding path
    pub fn new(path: PathBuf) -> Self {
        ParsedPdb {
            path,
            assembly_info: AssemblyInfo::default(),
            public_symbols: vec![],
            types: Default::default(),
            procedures: vec![],
            global_data: vec![],
            debug_modules: vec![],
            forward_references: vec![],
            version: Version::Other(0),
            guid: uuid::Uuid::nil(),
            age: 0,
            timestamp: 0,
            machine_type: None,
        }
    }
}

#[cfg(feature = "serde")]
fn serialize_uuid<S: serde::Serializer>(uuid: &uuid::Uuid, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(uuid.to_string().as_ref())
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum MachineType {
    /// The contents of this field are assumed to be applicable to any machine type.
    Unknown,
    /// Matsushita AM33
    Am33,
    /// x64
    Amd64,
    /// ARM little endian
    Arm,
    /// ARM64 little endian
    Arm64,
    /// ARM Thumb-2 little endian
    ArmNT,
    /// EFI byte code
    Ebc,
    /// Intel 386 or later processors and compatible processors
    X86,
    /// Intel Itanium processor family
    Ia64,
    /// Mitsubishi M32R little endian
    M32R,
    /// MIPS16
    Mips16,
    /// MIPS with FPU
    MipsFpu,
    /// MIPS16 with FPU
    MipsFpu16,
    /// Power PC little endian
    PowerPC,
    /// Power PC with floating point support
    PowerPCFP,
    /// MIPS little endian
    R4000,
    /// RISC-V 32-bit address space
    RiscV32,
    /// RISC-V 64-bit address space
    RiscV64,
    /// RISC-V 128-bit address space
    RiscV128,
    /// Hitachi SH3
    SH3,
    /// Hitachi SH3 DSP
    SH3DSP,
    /// Hitachi SH4
    SH4,
    /// Hitachi SH5
    SH5,
    /// Thumb
    Thumb,
    /// MIPS little-endian WCE v2
    WceMipsV2,
    /// Invalid value
    Invalid,
}

impl From<&pdb::MachineType> for MachineType {
    fn from(machine_type: &pdb::MachineType) -> Self {
        match machine_type {
            pdb::MachineType::Unknown => MachineType::Unknown,
            pdb::MachineType::Am33 => MachineType::Am33,
            pdb::MachineType::Amd64 => MachineType::Amd64,
            pdb::MachineType::Arm => MachineType::Arm,
            pdb::MachineType::Arm64 => MachineType::Arm64,
            pdb::MachineType::ArmNT => MachineType::ArmNT,
            pdb::MachineType::Ebc => MachineType::Ebc,
            pdb::MachineType::X86 => MachineType::X86,
            pdb::MachineType::Ia64 => MachineType::Ia64,
            pdb::MachineType::M32R => MachineType::M32R,
            pdb::MachineType::Mips16 => MachineType::Mips16,
            pdb::MachineType::MipsFpu => MachineType::MipsFpu,
            pdb::MachineType::MipsFpu16 => MachineType::MipsFpu16,
            pdb::MachineType::PowerPC => MachineType::PowerPC,
            pdb::MachineType::PowerPCFP => MachineType::PowerPCFP,
            pdb::MachineType::R4000 => MachineType::R4000,
            pdb::MachineType::RiscV32 => MachineType::RiscV32,
            pdb::MachineType::RiscV64 => MachineType::RiscV64,
            pdb::MachineType::RiscV128 => MachineType::RiscV128,
            pdb::MachineType::SH3 => MachineType::SH3,
            pdb::MachineType::SH3DSP => MachineType::SH3DSP,
            pdb::MachineType::SH4 => MachineType::SH4,
            pdb::MachineType::SH5 => MachineType::SH5,
            pdb::MachineType::Thumb => MachineType::Thumb,
            pdb::MachineType::WceMipsV2 => MachineType::WceMipsV2,
            pdb::MachineType::Invalid => MachineType::Invalid,
            other => panic!("unsupported machine type encountered: {:?}", other),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Version {
    V41,
    V50,
    V60,
    V70,
    V110,
    Other(u32),
}

impl From<&pdb::HeaderVersion> for Version {
    fn from(version: &pdb::HeaderVersion) -> Self {
        match version {
            pdb::HeaderVersion::V41 => Version::V41,
            pdb::HeaderVersion::V50 => Version::V50,
            pdb::HeaderVersion::V60 => Version::V60,
            pdb::HeaderVersion::V70 => Version::V70,
            pdb::HeaderVersion::V110 => Version::V110,
            pdb::HeaderVersion::OtherValue(other) => Version::Other(*other),
            other => panic!("unsupported PDB version encountered: {:?}", other),
        }
    }
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct AssemblyInfo {
    pub build_info: Option<BuildInfo>,
    pub compiler_info: Option<CompilerInfo>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct BuildInfo {
    arguments: Vec<String>,
}

impl TryFrom<(&pdb::BuildInfoSymbol, Option<&pdb::IdFinder<'_>>)> for BuildInfo {
    type Error = crate::error::Error;

    fn try_from(
        info: (&pdb::BuildInfoSymbol, Option<&pdb::IdFinder<'_>>),
    ) -> Result<Self, Self::Error> {
        let (symbol, finder) = info;
        if finder.is_none() {
            return Err(crate::error::Error::MissingDependency("IdFinder"));
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

                // TODO: Move this out into its own function for ID parsing
                let arguments: Vec<String> = argument_ids
                    .iter()
                    .map(|id| match id.parse().expect("failed to parse ID") {
                        pdb::IdData::String(s) => s.name.to_string().into_owned(),
                        other => panic!("unexpected ID type : {:?}", other),
                    })
                    .collect();

                return Ok(BuildInfo { arguments });
            }
            _ => unreachable!(),
        };

        Err(crate::error::Error::Unsupported("BuildInfo"))
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
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

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
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
            ..
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

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
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

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct DebugModule {
    name: String,
    object_file_name: String,
    source_files: Option<Vec<FileInfo>>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
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

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
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

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Data {
    pub name: String,

    pub is_global: bool,

    pub is_managed: bool,

    pub ty: TypeRef,

    pub offset: Option<usize>,
}

impl
    TryFrom<(
        pdb::DataSymbol<'_>,
        usize,
        Option<&pdb::AddressMap<'_>>,
        &HashMap<TypeIndexNumber, TypeRef>,
    )> for Data
{
    type Error = crate::error::Error;

    fn try_from(
        data: (
            pdb::DataSymbol<'_>,
            usize,
            Option<&pdb::AddressMap<'_>>,
            &HashMap<TypeIndexNumber, TypeRef>,
        ),
    ) -> Result<Self, Self::Error> {
        let (sym, base_address, address_map, parsed_types) = data;

        let pdb::DataSymbol {
            global,
            managed,
            type_index,
            offset,
            name,
        } = sym;

        let offset = address_map.and_then(|address_map| {
            offset
                .to_rva(address_map)
                .map(|rva| u32::from(rva) as usize + base_address)
        });

        let ty = Rc::clone(
            parsed_types
                .get(&type_index.0)
                .ok_or(Self::Error::UnresolvedType(type_index.0))?,
        );

        let data = Data {
            name: name.to_string().to_string(),
            is_global: global,
            is_managed: managed,
            ty,
            offset,
        };

        Ok(data)
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Procedure {
    pub name: String,

    pub signature: Option<String>,
    pub type_index: TypeIndexNumber,

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
            type_index: type_index.0,
            offset,
            len: len as usize,
            is_global: global,
            is_dpc: dpc,
            prologue_end: dbg_start_offset as usize,
            epilogue_start: dbg_end_offset as usize,
        }
    }
}
