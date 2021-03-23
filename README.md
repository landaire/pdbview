[![ezpdb API Documentation](https://docs.rs/ezpdb/badge.svg)](https://docs.rs/ezpdb)]
[![ezpdb on crates.io](https://img.shields.io/crates/v/ezpdb.svg)](https://crates.io/crates/ezpdb)
[![pdbview on crates.io](https://img.shields.io/crates/v/pdbview.svg)](https://crates.io/crates/pdbview)

# pdbview

dumps a lot of information from PDBs

## Installation

```
cargo install pdbview
```

## Usage

```
pdbview 0.1.0

USAGE:
    pdbview [FLAGS] [OPTIONS] <FILE>

FLAGS:
    -d, --debug      Print debug information
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --base-address <base-address>    Base address of module in-memory. If provided, all "offset" fields will be
                                         added to the provided base address
    -f, --format <format>                Output format type. Options include: plain, json [default: plain]

ARGS:
    <FILE>    PDB file to process
 
```

Example:

```
pdbview example.pdb
```

## Included Information

- Used modules (libraries)
- Source file names and checksums
- Compiler information
- Procedure information
- Type information
- Globals

## Example Output 

```
Assembly Info:
        Build Info:
        Compiler Info:
                Language: Link
                Flags:
                        Edit and continue:                       false
                        No debug info:                           false
                        Link-time codegen (LTCG):                false
                        No data align (/bzalign):                false
                        Manged code or data is present:          false
                        Security checks (/GS):                   false
                        Hot patching (/hotpatch):                false
                        CvtCIL:                                  false
                        Is MSIL .NET module:                     false
                        Compiled with /SDL:                      false
                        PGO (`/ltcg:pgo` or `pgo:`):             false
                        Is .exp module:                          false
                CPU type: Intel80386
                Frontend version: 0.0.0, QFE=0
                Backend version: 10.0.40219, QFE=1
                Version string: Microsoft (R) LINK
Public symbols:
        Offset     Name
        0x00006520 _ChromeMain
        0x00076480 _RelaunchChromeBrowserWithNewCommandLineIfNeeded
        0x020213C0 __ovly_debug_event
        0x02C8D7EC _nacl_global_xlate_base
        0x02C73640 _nacl_thread_ids
        0x02C63640 _nacl_user
        0x0241EA94 _p_thread_callback_dllmain_typical_entry
        0x00006580 _DllMain@12
        0x0241EAA8 ___xl_z
        0x0241EA90 ___xl_a
Procedures:
        Offset     Length     Prologue End    Epilogue Start  Name
        0x00006560 0x0000000B 0x00000000     0x0000000A     CrashOnProcessDetach
        0x00006570 0x0000000A 0x00000000     0x00000007     on_callback
        0x00006580 0x0000006C 0x00000003     0x00000068     DllMain
        0x00006520 0x00000034 0x00000007     0x00000030     ChromeMain
        0x020213C0 0x00000001 0x00000000     0x00000000     content::ContentMainDelegate::PreSandboxStartup
        0x00DFEF10 0x00000003 0x00000000     0x00000000     content::ContentMainDelegate::SandboxInitialized
        0x00DFEF10 0x00000003 0x00000000     0x00000000     content::ContentMainDelegate::ProcessExiting
        0x00001000 0x00000022 0x0000000A     0x0000001E     content::ContentMainDelegate::`scalar deleting destructor'
        0x01B22000 0x00000004 0x00000000     0x00000003     logging::LogMessage::stream
        0x004E0E10 0x00000003 0x00000002     0x00000002     logging::LogMessageVoidify::LogMessageVoidify
```
