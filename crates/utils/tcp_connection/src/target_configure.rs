#[derive(Default, Debug, Clone, Copy)]
pub struct ServerTargetConfig {
    /// Only process a single connection, then shut down the server.
    once: bool,

    /// Timeout duration in milliseconds. (0 is Closed)
    timeout: u64,
}

impl ServerTargetConfig {
    /// Set `once` to True
    /// This method configures the `once` field of `ServerTargetConfig`.
    pub fn once(mut self) -> Self {
        self.once = true;
        self
    }

    /// Set `timeout` to the given value
    /// This method configures the `timeout` field of `ServerTargetConfig`.
    pub fn timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set `once` to the given value
    /// This method configures the `once` field of `ServerTargetConfig`.
    pub fn set_once(&mut self, enable: bool) {
        self.once = enable;
    }

    /// Set `timeout` to the given value
    /// This method configures the `timeout` field of `ServerTargetConfig`.
    pub fn set_timeout(&mut self, timeout: u64) {
        self.timeout = timeout;
    }

    /// Check if the server is configured to process only a single connection.
    /// Returns `true` if the server will shut down after processing one connection.
    pub fn is_once(&self) -> bool {
        self.once
    }

    /// Get the current timeout value in milliseconds.
    /// Returns the timeout duration. A value of 0 indicates the connection is closed.
    pub fn get_timeout(&self) -> u64 {
        self.timeout
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ClientTargetConfig {}
