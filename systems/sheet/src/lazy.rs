#[macro_export]
macro_rules! lazy_node {
    ($($elem:expr),* $(,)?) => {
        vec![$($elem.to_string()),*]
    };
}

#[macro_export]
macro_rules! lazy_idx {
    ($idx:expr, $ver:expr) => {
        crate::index_source::IndexSource::new(false, $idx, $ver)
    };
}

#[macro_export]
macro_rules! lazy_ridx {
    ($idx:expr, $ver:expr) => {
        crate::index_source::IndexSource::new(true, $idx, $ver)
    };
}
