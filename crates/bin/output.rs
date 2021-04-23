use ezpdb::symbol_types::*;
use ezpdb::type_info::*;
use log::{debug, warn};
use std::io::{self, Write};

pub fn print_plain(output: &mut impl Write, pdb_info: &ParsedPdb) -> io::Result<()> {
    // region: Header info
    // Print header information
    writeln!(output, "{:?}:", &pdb_info.path)?;

    writeln!(output, "PDB Version: {:?}", pdb_info.version)?;
    writeln!(
        output,
        "Machine Type: {}",
        pdb_info
            .machine_type
            .as_ref()
            .map(|ty| format!("{:?}", ty))
            .unwrap_or_else(|| "Unknown".to_string())
    )?;

    writeln!(output, "Assembly Info:")?;

    writeln!(output, "\tBuild Info:")?;

    writeln!(output, "\tCompiler Info:")?;
    let width = 40usize;
    if let Some(compiler_info) = &pdb_info.assembly_info.compiler_info {
        writeln!(output, "\t\tLanguage: {}", compiler_info.language)?;

        let CompileFlags {
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
            "Managed code or data is present:",
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
        let CompilerVersion {
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

        let CompilerVersion {
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
    // endregion

    // region: Public symbols
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
    // endregion

    // region: Procedures
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
    // endregion

    // region: Data
    writeln!(output, "Globals:")?;
    writeln!(output, "\t{:<10} {:<10}", "Offset", "Name")?;

    for global in &pdb_info.global_data {
        write!(output, "\t")?;
        if let Some(offset) = global.offset {
            write!(output, "0x{:08X} ", offset)?;
        } else {
            write!(output, "{:<10} ", "")?;
        }
        writeln!(output, "{}", global.name)?;

        let ty: &Type = &*global.ty.as_ref().borrow();
        writeln!(output, "\t\tType: {}", format_type_name(ty))?;
        writeln!(output, "\t\tSize: 0x{:X}", ty.type_size(pdb_info))?;
        writeln!(output, "\t\tIs Managed: {}", global.is_managed)?;
    }
    // endregion

    // region: Types
    writeln!(output)?;
    writeln!(output, "Types:")?;

    let width = 20usize;
    for ty in pdb_info.types.values() {
        let ty: &Type = &*ty.as_ref().borrow();
        match ty {
            Type::Class(class) => {
                if class.properties.forward_reference {
                    continue;
                }

                writeln!(
                    output,
                    "\t{:width$} {} {}",
                    class.kind,
                    class.name,
                    class.unique_name.as_ref().map(String::as_ref).unwrap_or(""),
                    width = 10
                )?;
                writeln!(output, "\tSize: 0x{:X}", class.size)?;
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
                writeln!(output, "\tFields:")?;
                for field in &class.fields {
                    let field: &Type = &*field.as_ref().borrow();

                    match field {
                        Type::Member(member) => {
                            let member_ty: &Type = &*member.underlying_type.as_ref().borrow();
                            writeln!(
                                output,
                                "\t\t0x{:04X} {:width$} {}",
                                member.offset,
                                member.name,
                                format_type_name(member_ty),
                                width = width
                            )?;
                        }
                        Type::BaseClass(base) => {
                            writeln!(
                                output,
                                "\t\t0x{:04X} <BaseClass> {}",
                                base.offset,
                                format_type_name(&*base.base_class.as_ref().borrow())
                            )?;
                        }
                        Type::VirtualBaseClass(_) => {
                            // ignore
                        }
                        Type::Nested(_nested) => {
                            // writeln!(
                            //     output,
                            //     "\t\t (NestedType) {} {}",
                            //     nested.name,
                            //     format_type_name(&*nested.nested_type.as_ref().borrow())
                            // )?;
                        }
                        Type::Method(_) | Type::OverloadedMethod(_) => {
                            // ignore methods
                        }
                        Type::VTable(_) => {
                            // ignore vtable
                        }
                        Type::StaticMember(_) => {
                            // ignore
                        }
                        other => {
                            debug!("Unexpected field type present in class: {:?}", other)
                        }
                    }
                }
            }
            Type::Union(union) => {
                if union.properties.forward_reference {
                    continue;
                }

                writeln!(
                    output,
                    "\tUnion {} {}",
                    union.name,
                    union.unique_name.as_ref().map(String::as_ref).unwrap_or(""),
                )?;
                writeln!(output, "\tSize: 0x{:X}", union.size)?;
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
                writeln!(output, "\tFields:")?;
                for field in &union.fields {
                    let field: &Type = &*field.as_ref().borrow();

                    match field {
                        Type::Member(member) => {
                            let member_ty: &Type = &*member.underlying_type.as_ref().borrow();
                            writeln!(
                                output,
                                "\t\t0x{:04X} {:width$} {}",
                                member.offset,
                                member.name,
                                format_type_name(member_ty),
                                width = width
                            )?;
                        }
                        Type::BaseClass(base) => {
                            writeln!(
                                output,
                                "\t\t0x{:04X} <BaseClass> {}",
                                base.offset,
                                format_type_name(&*base.base_class.as_ref().borrow())
                            )?;
                        }
                        Type::VirtualBaseClass(_) => {
                            // ignore
                        }
                        Type::Nested(_nested) => {
                            // ignore nested types
                            // writeln!(
                            //     output,
                            //     "\t\t (NestedType) {} {}",
                            //     nested.name,
                            //     format_type_name(&*nested.nested_type.as_ref().borrow())
                            // )?;
                        }
                        Type::Method(_) | Type::OverloadedMethod(_) => {
                            // ignore methods
                        }
                        Type::VTable(_) => {
                            // ignore vtable
                        }
                        Type::StaticMember(_) => {
                            // ignore
                        }
                        other => {
                            debug!("Unexpected field type present in class: {:?}", other)
                        }
                    }
                }
            }
            _ => {
                continue;
            }
        }
        writeln!(output)?;
    }
    // endregion

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
            match pointer.underlying_type.as_ref() {
                Some(underlying_type) => {
                    format!("{}*", format_type_name(&*underlying_type.as_ref().borrow()))
                }
                None => "<UNRESOLVED_POINTER_TYPE>".to_string(),
            }
        }
        Type::Primitive(primitive) => match primitive.kind {
            PrimitiveKind::Void => "void".to_string(),
            PrimitiveKind::Char | PrimitiveKind::RChar => "char".to_string(),
            PrimitiveKind::UChar => "unsigned char".to_string(),

            PrimitiveKind::I8 => "int8_t".to_string(),
            PrimitiveKind::U8 => "uint8_t".to_string(),
            PrimitiveKind::I16 | PrimitiveKind::Short => "int16_t".to_string(),
            PrimitiveKind::U16 | PrimitiveKind::UShort => "uint16_t".to_string(),
            PrimitiveKind::I32 | PrimitiveKind::Long => "int32_t".to_string(),
            PrimitiveKind::U32 | PrimitiveKind::ULong => "uint32_t".to_string(),
            PrimitiveKind::I64 | PrimitiveKind::Quad => "int64_t".to_string(),
            PrimitiveKind::U64 | PrimitiveKind::UQuad => "uint64_t".to_string(),

            PrimitiveKind::F32 => "float".to_string(),
            PrimitiveKind::F64 => "double".to_string(),

            PrimitiveKind::Bool8 => "bool".to_string(),
            other => {
                format!("{}", other)
            }
        },
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
        Type::MemberFunction(member) => {
            format!(
                "{} (*function){}",
                format_type_name(&*member.return_type.as_ref().borrow()),
                member
                    .argument_list
                    .iter()
                    .fold(String::new(), |accum, argument| {
                        format!(
                            "{}{}{}",
                            &accum,
                            if accum.is_empty() { "" } else { "," },
                            format_type_name(&*argument.as_ref().borrow())
                        )
                    })
            )
        }
        other => panic!("unimplemented type format: {:?}", other),
    }
}

pub fn print_json(output: &mut impl Write, pdb_info: &ParsedPdb) -> io::Result<()> {
    write!(output, "{}", serde_json::to_string(pdb_info)?)
}
