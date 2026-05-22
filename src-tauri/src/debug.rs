pub fn log(scope: &str, message: impl AsRef<str>) {
    #[cfg(debug_assertions)]
    eprintln!("[QPaw debug][{scope}] {}", message.as_ref());
}

pub fn err(scope: &str, message: impl AsRef<str>) {
    #[cfg(debug_assertions)]
    eprintln!("[QPaw error][{scope}] {}", message.as_ref());
}
