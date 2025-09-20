#[macro_export]
macro_rules! camel_case {
    ($input:expr) => {{
        use string_proc::string_processer::StringProcesser;
        StringProcesser::from($input).to_camel_case()
    }};
}

#[macro_export]
macro_rules! upper_case {
    ($input:expr) => {{
        use string_proc::string_processer::StringProcesser;
        StringProcesser::from($input).to_upper_case()
    }};
}

#[macro_export]
macro_rules! lower_case {
    ($input:expr) => {{
        use string_proc::string_processer::StringProcesser;
        StringProcesser::from($input).to_lower_case()
    }};
}

#[macro_export]
macro_rules! title_case {
    ($input:expr) => {{
        use string_proc::string_processer::StringProcesser;
        StringProcesser::from($input).to_title_case()
    }};
}

#[macro_export]
macro_rules! dot_case {
    ($input:expr) => {{
        use string_proc::string_processer::StringProcesser;
        StringProcesser::from($input).to_dot_case()
    }};
}

#[macro_export]
macro_rules! snake_case {
    ($input:expr) => {{
        use string_proc::string_processer::StringProcesser;
        StringProcesser::from($input).to_snake_case()
    }};
}

#[macro_export]
macro_rules! kebab_case {
    ($input:expr) => {{
        use string_proc::string_processer::StringProcesser;
        StringProcesser::from($input).to_kebab_case()
    }};
}

#[macro_export]
macro_rules! pascal_case {
    ($input:expr) => {{
        use string_proc::string_processer::StringProcesser;
        StringProcesser::from($input).to_pascal_case()
    }};
}
