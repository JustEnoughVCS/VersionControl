pub struct FormatProcesser {
    content: Vec<String>,
}

impl From<String> for FormatProcesser {
    fn from(value: String) -> Self {
        Self {
            content: Self::process_string(value),
        }
    }
}

impl From<&str> for FormatProcesser {
    fn from(value: &str) -> Self {
        Self {
            content: Self::process_string(value.to_string()),
        }
    }
}

impl FormatProcesser {
    /// Process the string into an intermediate format
    fn process_string(input: String) -> Vec<String> {
        let mut result = String::new();
        let mut prev_space = false;

        for c in input.chars() {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' => {
                    result.push(c);
                    prev_space = false;
                }
                '_' | ',' | '.' | '-' | ' ' => {
                    if !prev_space {
                        result.push(' ');
                        prev_space = true;
                    }
                }
                _ => {}
            }
        }

        let mut processed = String::new();
        let mut chars = result.chars().peekable();

        while let Some(c) = chars.next() {
            processed.push(c);
            if let Some(&next) = chars.peek()
                && c.is_lowercase()
                && next.is_uppercase()
            {
                processed.push(' ');
            }
        }

        processed
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }

    /// Convert to camelCase format (brewCoffee)
    ///
    /// # Examples
    ///
    /// ```
    /// # use string_proc::format_processer::FormatProcesser;
    /// let processor = FormatProcesser::from("brew_coffee");
    /// assert_eq!(processor.to_camel_case(), "brewCoffee");
    /// ```
    pub fn to_camel_case(&self) -> String {
        let mut result = String::new();
        for (i, word) in self.content.iter().enumerate() {
            if i == 0 {
                result.push_str(&word.to_lowercase());
            } else {
                let mut chars = word.chars();
                if let Some(first) = chars.next() {
                    result.push_str(&first.to_uppercase().collect::<String>());
                    result.push_str(&chars.collect::<String>().to_lowercase());
                }
            }
        }
        result
    }

    /// Convert to PascalCase format (BrewCoffee)
    ///
    /// # Examples
    ///
    /// ```
    /// # use string_proc::format_processer::FormatProcesser;
    /// let processor = FormatProcesser::from("brew_coffee");
    /// assert_eq!(processor.to_pascal_case(), "BrewCoffee");
    /// ```
    pub fn to_pascal_case(&self) -> String {
        let mut result = String::new();
        for word in &self.content {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                result.push_str(&first.to_uppercase().collect::<String>());
                result.push_str(&chars.collect::<String>().to_lowercase());
            }
        }
        result
    }

    /// Convert to kebab-case format (brew-coffee)
    ///
    /// # Examples
    ///
    /// ```
    /// # use string_proc::format_processer::FormatProcesser;
    /// let processor = FormatProcesser::from("brew_coffee");
    /// assert_eq!(processor.to_kebab_case(), "brew-coffee");
    /// ```
    pub fn to_kebab_case(&self) -> String {
        self.content.join("-").to_lowercase()
    }

    /// Convert to snake_case format (brew_coffee)
    ///
    /// # Examples
    ///
    /// ```
    /// # use string_proc::format_processer::FormatProcesser;
    /// let processor = FormatProcesser::from("brewCoffee");
    /// assert_eq!(processor.to_snake_case(), "brew_coffee");
    /// ```
    pub fn to_snake_case(&self) -> String {
        self.content.join("_").to_lowercase()
    }

    /// Convert to dot.case format (brew.coffee)
    ///
    /// # Examples
    ///
    /// ```
    /// # use string_proc::format_processer::FormatProcesser;
    /// let processor = FormatProcesser::from("brew_coffee");
    /// assert_eq!(processor.to_dot_case(), "brew.coffee");
    /// ```
    pub fn to_dot_case(&self) -> String {
        self.content.join(".").to_lowercase()
    }

    /// Convert to Title Case format (Brew Coffee)
    ///
    /// # Examples
    ///
    /// ```
    /// # use string_proc::format_processer::FormatProcesser;
    /// let processor = FormatProcesser::from("brew_coffee");
    /// assert_eq!(processor.to_title_case(), "Brew Coffee");
    /// ```
    pub fn to_title_case(&self) -> String {
        let mut result = String::new();
        for word in &self.content {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                result.push_str(&first.to_uppercase().collect::<String>());
                result.push_str(&chars.collect::<String>().to_lowercase());
            }
            result.push(' ');
        }
        result.pop();
        result
    }

    /// Convert to lower case format (brew coffee)
    ///
    /// # Examples
    ///
    /// ```
    /// # use string_proc::format_processer::FormatProcesser;
    /// let processor = FormatProcesser::from("BREW COFFEE");
    /// assert_eq!(processor.to_lower_case(), "brew coffee");
    /// ```
    pub fn to_lower_case(&self) -> String {
        self.content.join(" ").to_lowercase()
    }

    /// Convert to UPPER CASE format (BREW COFFEE)
    ///
    /// # Examples
    ///
    /// ```
    /// # use string_proc::format_processer::FormatProcesser;
    /// let processor = FormatProcesser::from("brew coffee");
    /// assert_eq!(processor.to_upper_case(), "BREW COFFEE");
    /// ```
    pub fn to_upper_case(&self) -> String {
        self.content.join(" ").to_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use crate::format_processer::FormatProcesser;

    #[test]
    fn test_processer() {
        let test_cases = vec![
            ("brew_coffee", "brewCoffee"),
            ("brew, coffee", "brewCoffee"),
            ("brew-coffee", "brewCoffee"),
            ("Brew.Coffee", "brewCoffee"),
            ("bRewCofFee", "bRewCofFee"),
            ("brewCoffee", "brewCoffee"),
            ("b&rewCoffee", "brewCoffee"),
            ("BrewCoffee", "brewCoffee"),
            ("brew.coffee", "brewCoffee"),
            ("Brew_Coffee", "brewCoffee"),
            ("BREW COFFEE", "brewCoffee"),
        ];

        for (input, expected) in test_cases {
            let processor = FormatProcesser::from(input);
            assert_eq!(
                processor.to_camel_case(),
                expected,
                "Failed for input: '{}'",
                input
            );
        }
    }

    #[test]
    fn test_conversions() {
        let processor = FormatProcesser::from("brewCoffee");

        assert_eq!(processor.to_upper_case(), "BREW COFFEE");
        assert_eq!(processor.to_lower_case(), "brew coffee");
        assert_eq!(processor.to_title_case(), "Brew Coffee");
        assert_eq!(processor.to_dot_case(), "brew.coffee");
        assert_eq!(processor.to_snake_case(), "brew_coffee");
        assert_eq!(processor.to_kebab_case(), "brew-coffee");
        assert_eq!(processor.to_pascal_case(), "BrewCoffee");
        assert_eq!(processor.to_camel_case(), "brewCoffee");
    }
}
