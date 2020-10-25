use std::io::{self, Write};
use crate::typeinfo::ParsedPdb;

pub fn print_plain(output: &mut impl Write, pdb_info: &ParsedPdb<'_>) -> io::Result<()> {
    write!(output, "{:#X?}", pdb_info)
}

pub fn print_json(output: &mut impl Write, pdb_info: &ParsedPdb<'_>) -> io::Result<()> {
    Ok(())
}