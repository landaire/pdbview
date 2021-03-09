use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParsingError {
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
}
