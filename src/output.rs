use crate::typeinfo::ParsedPdb;
use std::io::{self, Write};

pub fn print_plain(output: &mut impl Write, pdb_info: &ParsedPdb) -> io::Result<()> {
    write!(output, "{:#X?}", pdb_info)
}

pub fn print_json(output: &mut impl Write, pdb_info: &ParsedPdb) -> io::Result<()> {
    write!(output, "{}", serde_json::to_string(pdb_info)?)
}
