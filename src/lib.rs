// Feature `vcs`
#[cfg(feature = "vcs")]
pub mod vcs {
    extern crate vcs;
    pub use vcs::*;
}

pub mod utils {
    // Feature `tcp_connection`
    #[cfg(feature = "tcp_connection")]
    pub mod tcp_connection {
        extern crate tcp_connection;
        pub use tcp_connection::*;
    }

    // Feature `string_proc`
    #[cfg(feature = "string_proc")]
    pub mod string_proc {
        extern crate string_proc;
        pub use string_proc::*;
    }

    // Feature `cfg_file`
    #[cfg(feature = "cfg_file")]
    pub mod cfg_file {
        extern crate cfg_file;
        pub use cfg_file::*;
    }
}

pub mod prelude {
    #[cfg(feature = "vcs")]
    pub use super::vcs::*;

    #[cfg(feature = "tcp_connection")]
    pub use super::utils::tcp_connection::*;

    #[cfg(feature = "string_proc")]
    pub use super::utils::string_proc::*;

    #[cfg(feature = "cfg_file")]
    pub use super::utils::cfg_file::*;
}
