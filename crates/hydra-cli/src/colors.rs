pub fn green(s: &str) -> String {
    format!("\x1b[32m{}\x1b[0m", s)
}

pub fn red(s: &str) -> String {
    format!("\x1b[31m{}\x1b[0m", s)
}

pub fn blue(s: &str) -> String {
    format!("\x1b[34m{}\x1b[0m", s)
}

pub fn yellow(s: &str) -> String {
    format!("\x1b[33m{}\x1b[0m", s)
}

pub fn bold(s: &str) -> String {
    format!("\x1b[1m{}\x1b[0m", s)
}

pub fn dim(s: &str) -> String {
    format!("\x1b[2m{}\x1b[0m", s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_wraps_with_ansi() {
        let result = green("ok");
        assert!(result.contains("ok"));
        assert!(result.starts_with("\x1b[32m"));
        assert!(result.ends_with("\x1b[0m"));
    }

    #[test]
    fn bold_wraps_with_ansi() {
        let result = bold("title");
        assert!(result.contains("title"));
        assert!(result.starts_with("\x1b[1m"));
    }

    #[test]
    fn all_colors_contain_reset() {
        for f in &[green, red, blue, yellow, bold, dim] {
            let result = f("test");
            assert!(result.ends_with("\x1b[0m"), "Missing reset code");
        }
    }
}
