pub mod macros;
pub mod string_processer;

#[cfg(test)]
mod tests {
    use crate::string_processer::StringProcesser;

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
            let processor = StringProcesser::from(input);
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
        let processor = StringProcesser::from("brewCoffee");

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
