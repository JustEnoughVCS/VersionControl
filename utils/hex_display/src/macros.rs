#[macro_export]
macro_rules! hex_display {
    ($slice:expr) => {
        hex_display::hex_display_slice($slice)
    };
    ($vec:expr) => {
        hex_display::hex_display_vec($vec)
    };
}
