use crate::typeinfo::ParsedPdb;
use std::io::{self, Write};

pub fn print_plain(output: &mut impl Write, pdb_info: &ParsedPdb) -> io::Result<()> {
    // Print header information
    writeln!(output, "{:?}:", &pdb_info.path)?;

    writeln!(output, "Assembly Info:")?;

    writeln!(output, "\tBuild Info:")?;

    writeln!(output, "\tCompiler Info:")?;
    let width = 40usize;
    if let Some(compiler_info) = &pdb_info.assembly_info.compiler_info {
        writeln!(output, "\t\tLanguage: {}", compiler_info.language)?;

        let crate::typeinfo::CompileFlags {
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
        } = compiler_info.flags;
        writeln!(output, "\t\tFlags:")?;

        writeln!(
            output,
            "\t\t\t{:width$} {}",
            "Edit and continue:",
            edit_and_continue,
            width = width
        )?;
        writeln!(
            output,
            "\t\t\t{:width$} {}",
            "No debug info:",
            no_debug_info,
            width = width
        )?;
        writeln!(
            output,
            "\t\t\t{:width$} {}",
            "Link-time codegen (LTCG):",
            link_time_codegen,
            width = width
        )?;
        writeln!(
            output,
            "\t\t\t{:width$} {}",
            "No data align (/bzalign):",
            no_data_align,
            width = width
        )?;
        writeln!(
            output,
            "\t\t\t{:width$} {}",
            "Manged code or data is present:",
            managed,
            width = width
        )?;
        writeln!(
            output,
            "\t\t\t{:width$} {}",
            "Security checks (/GS):",
            security_checks,
            width = width
        )?;
        writeln!(
            output,
            "\t\t\t{:width$} {}",
            "Hot patching (/hotpatch):",
            hot_patch,
            width = width
        )?;
        writeln!(
            output,
            "\t\t\t{:width$} {}",
            "CvtCIL:",
            cvtcil,
            width = width
        )?;
        writeln!(
            output,
            "\t\t\t{:width$} {}",
            "Is MSIL .NET module:",
            msil_module,
            width = width
        )?;
        writeln!(
            output,
            "\t\t\t{:width$} {}",
            "Compiled with /SDL:",
            sdl,
            width = width
        )?;
        writeln!(
            output,
            "\t\t\t{:width$} {}",
            "PGO (`/ltcg:pgo` or `pgo:`):",
            pgo,
            width = width
        )?;
        writeln!(
            output,
            "\t\t\t{:width$} {}",
            "Is .exp module:",
            pgo,
            width = width
        )?;
        writeln!(output, "\t\tCPU type: {}", compiler_info.cpu_type,)?;
        let crate::typeinfo::CompilerVersion {
            major,
            minor,
            build,
            qfe,
        } = compiler_info.frontend_version;
        writeln!(
            output,
            "\t\tFrontend version: {}.{}.{}, QFE={}",
            major,
            minor,
            build,
            qfe.map(|qfe| format!("{}", qfe))
                .unwrap_or_else(|| "None".to_string())
        )?;

        let crate::typeinfo::CompilerVersion {
            major,
            minor,
            build,
            qfe,
        } = compiler_info.backend_version;
        writeln!(
            output,
            "\t\tBackend version: {}.{}.{}, QFE={}",
            major,
            minor,
            build,
            qfe.map(|qfe| format!("{}", qfe))
                .unwrap_or_else(|| "None".to_string())
        )?;
        writeln!(
            output,
            "\t\tVersion string: {}",
            compiler_info.version_string
        )?;
    }

    writeln!(output, "Public symbols:")?;
    writeln!(output, "\t{:<10} Name", "Offset")?;
    for symbol in &pdb_info.public_symbols {
        write!(output, "\t")?;
        if let Some(offset) = symbol.offset {
            write!(output, "0x{:08X} ", offset)?;
        } else {
            write!(output, "{:<10} ", "")?;
        }
        writeln!(output, "{}", symbol.name)?;
    }

    writeln!(output, "Procedures:")?;
    writeln!(
        output,
        "\t{:<10} {:<10} {:<15} {:<15} {:<10}",
        "Offset", "Length", "Prologue End", "Epilogue Start", "Name"
    )?;

    for procedure in &pdb_info.procedures {
        write!(output, "\t")?;
        if let Some(offset) = procedure.offset {
            write!(output, "0x{:08X} ", offset)?;
        } else {
            write!(output, "{:<10} ", "")?;
        }

        write!(output, "0x{:08X} ", procedure.len)?;
        write!(
            output,
            "{:<15}",
            format!("0x{:08X} ", procedure.prologue_end)
        )?;
        write!(
            output,
            "{:<15}",
            format!("0x{:08X} ", procedure.epilogue_start)
        )?;
        writeln!(output, "{}", procedure.name)?;
    }

    Ok(())
}

pub fn print_json(output: &mut impl Write, pdb_info: &ParsedPdb) -> io::Result<()> {
    write!(output, "{}", serde_json::to_string(pdb_info)?)
}
