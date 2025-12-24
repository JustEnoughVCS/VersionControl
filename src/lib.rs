/// Data
pub mod data;

// Feature `vcs`
#[cfg(feature = "vcs")]
pub mod vcs {
    pub extern crate vcs_data;
    pub use vcs_data::*;

    pub extern crate vcs_actions;
    pub use vcs_actions::*;

    pub extern crate vcs_docs;
    pub use vcs_docs::*;
}

pub mod system {
    pub mod action_system {
        pub use action_system::*;
    }
}

pub mod utils {
    // Feature `cfg_file`
    #[cfg(feature = "cfg_file")]
    pub mod cfg_file {
        extern crate cfg_file;
        pub use cfg_file::*;
    }

    // Feature `data_struct`
    #[cfg(feature = "data_struct")]
    pub mod data_struct {
        extern crate data_struct;
        pub use data_struct::*;
    }

    // Feature `sha1_hash`
    #[cfg(feature = "sha1_hash")]
    pub mod sha1_hash {
        extern crate sha1_hash;
        pub use sha1_hash::*;
    }

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
}
