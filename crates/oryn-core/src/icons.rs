pub fn icon_key(name: &str, is_dir: bool) -> &'static str {
    if name == ".." {
        return "parent";
    }
    if is_dir {
        return "folder";
    }
    let lower = name.to_lowercase();
    if lower.is_empty() {
        return "doc";
    }
    if let Some(key) = exact(&lower) {
        return key;
    }
    for (prefix, key) in [
        ("dockerfile.", "docker"),
        ("license.", "license"),
        ("licence.", "license"),
    ] {
        if lower.starts_with(prefix) {
            return key;
        }
    }
    match lower.rfind('.') {
        Some(i) if i > 0 && i < lower.len() - 1 => extension(&lower[i + 1..]).unwrap_or("doc"),
        _ => "doc",
    }
}

fn exact(name: &str) -> Option<&'static str> {
    Some(match name {
        "dockerfile" | "docker-compose.yml" | "docker-compose.yaml" => "docker",
        "package.json" | "package-lock.json" => "pkgJson",
        "vite.config.js" | "vite.config.ts" | "vite.config.mjs" | "vite.config.cjs" => "vite",
        ".gitignore" | ".gitattributes" | ".gitmodules" => "git",
        "license" | "licence" | "copying" => "license",
        _ => return None,
    })
}

fn extension(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "swift" => "swift",
        "go" | "gomod" | "gowork" => "go",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => "cpp",
        "cs" | "csx" => "csharp",
        "java" | "jar" | "class" => "java",
        "kt" | "kts" => "kotlin",
        "php" | "phtml" | "php4" | "php5" | "php7" | "php8" => "php",
        "rb" | "erb" | "gemspec" | "rake" => "ruby",
        "dart" => "dart",
        "vue" => "vue",
        "svelte" => "svelte",
        "lua" => "lua",
        "graphql" | "gql" => "graphql",
        "sol" => "solidity",
        "rs" => "rust",
        "py" | "pyw" | "ipynb" => "py",
        "js" | "mjs" | "cjs" => "js",
        "ts" | "mts" | "cts" => "ts",
        "jsx" | "tsx" => "react",
        "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" => "sh",
        "r" | "scala" | "sc" | "ex" | "exs" | "erl" | "hs" | "lhs" | "clj" | "cljs" | "cljc"
        | "edn" | "pl" | "pm" => "code",
        "md" | "markdown" | "mdown" | "mkd" => "md",
        "yml" | "yaml" => "yaml",
        "json" | "json5" | "jsonc" => "json",
        "css" | "scss" | "sass" | "less" | "styl" => "css",
        "html" | "htm" | "xhtml" => "html",
        "toml" | "ini" | "env" | "conf" | "cfg" | "properties" => "config",
        "sql" | "sqlite" | "sqlite3" | "db" | "pgsql" | "mysql" | "prisma" => "sql",
        "jpg" | "jpeg" | "png" | "gif" | "svg" | "webp" | "heic" | "heif" | "bmp" | "ico"
        | "tiff" | "tif" | "raw" | "cr2" | "nef" | "arw" | "dng" | "avif" | "psd" | "ai"
        | "eps" | "jfif" | "pjpeg" | "pjp" => "img",
        "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" | "mp4" | "mkv" | "avi" | "mov" | "webm" => {
            "media"
        }
        "ttf" | "otf" | "woff" | "woff2" | "eot" => "font",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "tgz" => "archive",
        "bak" | "backup" | "bkp" | "old" | "orig" | "save" | "tmp_bak" => "backup",
        "pdf" => "pdf",
        _ => return None,
    })
}

pub fn icon_color(key: &str) -> Option<u32> {
    Some(match key {
        "md" => 0x38bdf8,
        "yaml" => 0xc084fc,
        "js" => 0xf59e0b,
        "ts" => 0x3b82f6,
        "react" => 0x00d8ff,
        "json" => 0x38bdf8,
        "pkgJson" => 0x2dd4bf,
        "css" => 0x60a5fa,
        "git" => 0xf87171,
        "license" => 0x94a3b8,
        "vite" => 0xa855f7,
        "html" => 0xf97316,
        "rust" => 0xfb923c,
        "py" => 0x3b82f6,
        "sh" => 0x4ade80,
        "config" => 0xa1a1aa,
        "swift" => 0xf05138,
        "go" => 0x00add8,
        "c" => 0x659ad2,
        "cpp" => 0x519aba,
        "csharp" => 0xa855f7,
        "java" => 0xf58220,
        "kotlin" => 0x7f52ff,
        "php" => 0x777bb4,
        "ruby" => 0xef4444,
        "dart" => 0x0175c2,
        "vue" => 0x41b883,
        "svelte" => 0xff3e00,
        "lua" => 0x3b82f6,
        "docker" => 0x2496ed,
        "graphql" => 0xe10098,
        "solidity" => 0x627eea,
        "media" => 0xf43f5e,
        "font" => 0xec4899,
        "doc" => 0xa0a0a5,
        "pdf" => 0xff453a,
        "img" => 0xc084fc,
        "archive" => 0xffd60a,
        "backup" => 0x94a3b8,
        "code" => 0x34c759,
        "sql" => 0xf472b6,
        "parent" => 0x8e8e93,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_web_icon_registry() {
        assert_eq!(icon_key("..", true), "parent");
        assert_eq!(icon_key("src", true), "folder");
        assert_eq!(icon_key("Dockerfile", false), "docker");
        assert_eq!(icon_key("Dockerfile.dev", false), "docker");
        assert_eq!(icon_key("package.json", false), "pkgJson");
        assert_eq!(icon_key("LICENSE", false), "license");
        assert_eq!(icon_key("main.rs", false), "rust");
        assert_eq!(icon_key("App.TSX", false), "react");
        assert_eq!(icon_key("Frame 12.png", false), "img");
        assert_eq!(icon_key("clip.mov", false), "media");
        assert_eq!(icon_key("notes", false), "doc");
        assert_eq!(icon_key(".env", false), "doc");
    }

    #[test]
    fn every_key_but_folder_has_a_colour() {
        for key in ["parent", "doc", "img", "pdf", "rust", "archive"] {
            assert!(icon_color(key).is_some(), "{key}");
        }
        assert_eq!(icon_color("folder"), None);
    }
}
