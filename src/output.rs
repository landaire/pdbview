use crate::symbol_types::ParsedPdb;
use crate::type_info::Type;
use log::{debug, warn};
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

        let crate::symbol_types::CompileFlags {
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
            exp_module,
            width = width
        )?;
        writeln!(output, "\t\tCPU type: {}", compiler_info.cpu_type,)?;
        let crate::symbol_types::CompilerVersion {
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

        let crate::symbol_types::CompilerVersion {
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

    writeln!(output, "Types:")?;

    let width = 20usize;
    for ty in pdb_info.types.values() {
        use crate::type_info::*;

        let ty: &Type = &*ty.as_ref().borrow();
        match ty {
            Type::Class(class) => {
                if class.properties.forward_reference {
                    continue;
                }

                writeln!(
                    output,
                    "\t\t{:width$} {} {}",
                    class.kind,
                    class.name,
                    class.unique_name.as_ref().map(String::as_ref).unwrap_or(""),
                    width = 10
                )?;
                // writeln!(
                //     output,
                //     "\t\t{:width$} {}",
                //     "Name:",
                //     class.name,
                //     width = width
                // )?;
                // writeln!(
                //     output,
                //     "\t\t{:width$} {}",
                //     "Unique name:",
                //     class.unique_name.as_ref().map(String::as_ref).unwrap_or(""),
                //     width = width
                // )?;
                writeln!(output, "\t\tFields:")?;
                for field in &class.fields {
                    let field: &Type = &*field.as_ref().borrow();

                    match field {
                        Type::Member(member) => {
                            let member_ty: &Type = &*member.underlying_type.as_ref().borrow();
                            writeln!(
                                output,
                                "\t\t\t0x{:X} {:width$} {}",
                                member.offset,
                                member.name,
                                format_type_name(member_ty),
                                width = width
                            )?;
                        }
                        Type::BaseClass(base) => {
                            writeln!(
                                output,
                                "\t\t\t0x{:X} <BaseClass>  {}",
                                base.offset,
                                format_type_name(&*base.base_class.as_ref().borrow())
                            )?;
                        }
                        other => debug!("Unexpected field type present in class: {:?}", other),
                    }
                }
            }
            Type::Union(union) => {}
            _ => {
                continue;
            }
        }
        writeln!(output);
    }

    Ok(())
}

fn format_type_name(ty: &Type) -> String {
    match ty {
        Type::Class(class) => class.name.clone(),
        Type::Union(union) => union.name.clone(),
        Type::Array(array) => format!(
            "{}{}",
            format_type_name(&*array.element_type.as_ref().borrow()),
            array
                .dimensions_elements
                .iter()
                .fold(String::new(), |accum, dimension| format!(
                    "{}[0x{:X}]",
                    accum, dimension
                ))
        ),
        Type::Pointer(pointer) => {
            // TODO: Attributes
            format!(
                "{}*",
                format_type_name(&*pointer.underlying_type.as_ref().unwrap().as_ref().borrow())
            )
        }
        Type::Primitive(primitive) => format!("{}", primitive.kind),
        Type::Modifier(modifier) => format_type_name(&*modifier.underlying_type.as_ref().borrow()),
        Type::Bitfield(bitfield) => format!(
            "{}:{}",
            format_type_name(&*bitfield.underlying_type.as_ref().borrow()),
            bitfield.len
        ),
        Type::Procedure(proc) => format!(
            "{} (*function){}",
            format_type_name(&*proc.return_type.as_ref().unwrap().as_ref().borrow()),
            proc.argument_list
                .iter()
                .fold(String::new(), |accum, argument| {
                    format!(
                        "{}{}{}",
                        &accum,
                        if accum.is_empty() { "" } else { "," },
                        format_type_name(&*argument.as_ref().borrow())
                    )
                })
        ),
        Type::Enumeration(e) => e.name.clone(),
        other => panic!("unimplemented type format: {:?}", other),
    }
}

pub fn print_json(output: &mut impl Write, pdb_info: &ParsedPdb) -> io::Result<()> {
    write!(output, "{}", serde_json::to_string(pdb_info)?)
}
