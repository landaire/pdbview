use thiserror::Error;

use crate::symbol_types::TypeIndexNumber;

#[derive(Error, Debug)]
pub enum Error {
    #[error("the PDB parsing library encountered an error: {0}")]
    PdbCrateError(#[from] pdb::Error),

    #[error("dependency `{0}` required for parsing is unavailable")]
    MissingDependency(&'static str),

    #[error("functionality `{0}` is currently unsupported")]
    Unsupported(&'static str),

    #[error("a forward reference implmentation is needed")]
    NeedForwardReferenceImplementation,

    #[error("type `{0}` was not handled")]
    UnhandledType(String),

    #[error("IO error occurred: {0}")]
    IoError(#[from] std::io::Error),

    #[error("could not resolve type index {0}")]
    UnresolvedType(TypeIndexNumber),
}
