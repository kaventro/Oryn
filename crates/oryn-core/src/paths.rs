pub fn clean_verbatim_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", rest)
    } else if let Some(rest) = path.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::clean_verbatim_path;

    #[test]
    fn strips_verbatim_prefixes() {
        assert_eq!(clean_verbatim_path(r"\\?\C:\Users"), r"C:\Users");
        assert_eq!(
            clean_verbatim_path(r"\\?\UNC\server\share"),
            r"\\server\share"
        );
        assert_eq!(clean_verbatim_path("/home/dev"), "/home/dev");
    }
}
