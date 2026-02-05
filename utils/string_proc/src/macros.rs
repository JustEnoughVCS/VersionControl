/// Convert to camelCase format (brewCoffee)
///
/// # Examples
///
/// ```
/// # use string_proc::camel_case;
/// assert_eq!(camel_case!("brew_coffee"), "brewCoffee");
/// ```
#[macro_export]
macro_rules! camel_case {
    ($input:expr) => {{
        use string_proc::format_processer::FormatProcesser;
        FormatProcesser::from($input).to_camel_case()
    }};
}

/// Convert to UPPER CASE format (BREW COFFEE)
///
/// # Examples
///
/// ```
/// # use string_proc::upper_case;
/// assert_eq!(upper_case!("brew coffee"), "BREW COFFEE");
/// ```
#[macro_export]
macro_rules! upper_case {
    ($input:expr) => {{
        use string_proc::format_processer::FormatProcesser;
        FormatProcesser::from($input).to_upper_case()
    }};
}

/// Convert to lower case format (brew coffee)
///
/// # Examples
///
/// ```
/// # use string_proc::lower_case;
/// assert_eq!(lower_case!("BREW COFFEE"), "brew coffee");
/// ```
#[macro_export]
macro_rules! lower_case {
    ($input:expr) => {{
        use string_proc::format_processer::FormatProcesser;
        FormatProcesser::from($input).to_lower_case()
    }};
}

/// Convert to Title Case format (Brew Coffee)
///
/// # Examples
///
/// ```
/// # use string_proc::title_case;
/// assert_eq!(title_case!("brew_coffee"), "Brew Coffee");
/// ```
#[macro_export]
macro_rules! title_case {
    ($input:expr) => {{
        use string_proc::format_processer::FormatProcesser;
        FormatProcesser::from($input).to_title_case()
    }};
}

/// Convert to dot.case format (brew.coffee)
///
/// # Examples
///
/// ```
/// # use string_proc::dot_case;
/// assert_eq!(dot_case!("brew_coffee"), "brew.coffee");
/// ```
#[macro_export]
macro_rules! dot_case {
    ($input:expr) => {{
        use string_proc::format_processer::FormatProcesser;
        FormatProcesser::from($input).to_dot_case()
    }};
}

/// Convert to snake_case format (brew_coffee)
///
/// # Examples
///
/// ```
/// # use string_proc::snake_case;
/// assert_eq!(snake_case!("brewCoffee"), "brew_coffee");
/// ```
#[macro_export]
macro_rules! snake_case {
    ($input:expr) => {{
        use string_proc::format_processer::FormatProcesser;
        FormatProcesser::from($input).to_snake_case()
    }};
}

/// Convert to kebab-case format (brew-coffee)
///
/// # Examples
///
/// ```
/// # use string_proc::kebab_case;
/// assert_eq!(kebab_case!("brew_coffee"), "brew-coffee");
/// ```
#[macro_export]
macro_rules! kebab_case {
    ($input:expr) => {{
        use string_proc::format_processer::FormatProcesser;
        FormatProcesser::from($input).to_kebab_case()
    }};
}

/// Convert to PascalCase format (BrewCoffee)
///
/// # Examples
///
/// ```
/// # use string_proc::pascal_case;
/// assert_eq!(pascal_case!("brew_coffee"), "BrewCoffee");
/// ```
#[macro_export]
macro_rules! pascal_case {
    ($input:expr) => {{
        use string_proc::format_processer::FormatProcesser;
        FormatProcesser::from($input).to_pascal_case()
    }};
}
