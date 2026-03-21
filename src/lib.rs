/// Data
pub mod data;

// Feature `lib`
#[cfg(feature = "lib")]
pub mod system {
    pub mod asset_system {
        pub use asset_system::*;
    }

    pub mod sheet_system {
        pub use sheet_system::*;
    }

    pub mod config_system {
        pub use config_system::*;
    }

    pub mod constants {
        pub use constants::*;
    }

    pub mod space {
        pub use framework::space::*;
        pub use framework::space_macro::*;
    }

    pub mod workspace {
        pub use workspace_system::*;
    }

    pub mod vault {
        pub use vault_system::*;
    }
}

pub mod utils {
    // Feature `data_struct`
    #[cfg(feature = "data_struct")]
    pub mod data_struct {
        extern crate data_struct;
        pub use data_struct::*;
    }

    // Feature `hex_display`
    #[cfg(feature = "hex_display")]
    pub mod hex_display {
        extern crate hex_display;
        pub use hex_display::*;
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
}
