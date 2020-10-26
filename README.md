# pdbview

dumps a lot of information from PDBs

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

## TODO

- Type information
- Global data information (depends on Types)
