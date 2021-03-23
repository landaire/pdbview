[![API Documentation](https://docs.rs/ezpdb/badge.svg)](https://docs.rs/ezpdb)]
[![crates.io](https://img.shields.io/crates/v/ezpdb.svg)](https://crates.io/crates/ezpdb)

# ezpdb

A high-level library for the fantastic [`pdb`](https://crates.io/crates/pdb) crate

## Goal

The `pdb` crate provides a low-level interface for interacting with PDB files. While the crate provides very good and useful information, it may not be the easiest solution for looking at the whole slice of a PDB. From the `pdb` crate's README:

>`pdb`'s design objectives are similar to
>[`gimli`](https://github.com/gimli-rs/gimli):
>
>* `pdb` works with the original data as it's formatted on-disk as long as
>  possible.
>
>* `pdb` parses only what you ask.
>
>* `pdb` can read PDBs anywhere. There's no dependency on Windows, on the
>  [DIA SDK](https://msdn.microsoft.com/en-us/library/x93ctkx8.aspx), or on
>  the target's native byte ordering.

The first two bullet points in particular can be particularly cumbersome if your application needs to iterate over every type, find one of a particular size or containing a particular field, and perform some operations on it. While you absolutely *should* care about speed and not performing unnecessary work, sometimes the reality is that you simply *don't*. This crate is for those scenarios where you just want to hack on some data and speed is not a concern.

The biggest differences are that `ezpdb` takes care of building the type heirarchy and copies all necessary information for ease of use.

## Usage

```rust
let parsed_pdb = ezpdb::parse_pdb(&opt.file, opt.base_address)?;
println!("{:?}", parsed_pdb.assembly_info);
```
