#[derive(Default, Clone, Eq, PartialEq)]
pub struct TcpTargetError {
    msg: String,
}

impl<'a> std::fmt::Display for TcpTargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl<'a> From<&'a str> for TcpTargetError {
    fn from(value: &'a str) -> Self {
        Self {
            msg: value.to_string(),
        }
    }
}

impl<'a> From<String> for TcpTargetError {
    fn from(value: String) -> Self {
        Self { msg: value }
    }
}
