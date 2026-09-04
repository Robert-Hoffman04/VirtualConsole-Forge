use thiserror::Error;

#[derive(Debug, Error)]
pub enum VcError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse registry: {0}")]
    RegistryParse(#[from] serde_json::Error),

    #[error("unknown core id: {0}")]
    UnknownCore(String),

    #[error("ROM validation failed: {0}")]
    InvalidRom(String),

    #[error("config blob malformed: {0}")]
    InvalidConfig(String),

    #[error("banner generation failed: {0}")]
    BannerError(String),

    #[error("donor WAD error: {0}")]
    DonorWad(String),

    #[error("donor key error: {0}")]
    DonorKey(String),

    #[error("image processing error: {0}")]
    Image(#[from] image::ImageError),

    #[error("WAD assembly failed: {0}")]
    WadAssembly(String),
}
