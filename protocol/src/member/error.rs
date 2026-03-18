#[derive(Debug, thiserror::Error)]
pub enum EMailAddressParseError {
    #[error("Invalid email format")]
    InvalidFormat,

    #[error("Account or domain cannot be empty")]
    EmptyPart,
}
