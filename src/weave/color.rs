

pub fn green(s: &str) -> String {
    format!("\x1b[32m{s}\x1b[0m")
}

pub fn red(s: &str) -> String {
    format!("\x1b[31m{s}\x1b[0m")
}