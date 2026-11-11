//! Placeholders in commands, URLs, arguments and env values, filled in as the prototype does: `${dir}` is the
//! project folder, `${url}` its website URL, and a `~` that starts a word (`~`, `~/…`) the home folder.

use std::path::Path;

pub struct Vars<'a> {
    pub dir: &'a Path,
    pub url: Option<&'a str>,
    pub home: Option<&'a Path>,
}

impl Vars<'_> {
    /// `${url}` stays as written when the project has no URL; `~` stays without a home folder.
    pub fn fill(&self, text: &str) -> String {
        let mut filled = text.replace("${dir}", &self.dir.to_string_lossy());
        if let Some(url) = self.url {
            filled = filled.replace("${url}", url);
        }
        match self.home {
            Some(home) => expand_tilde(&filled, &home.to_string_lossy()),
            None => filled,
        }
    }
}

/// Replaces each `~` that follows the start, a space or a quote and comes before a `/` or the end.
fn expand_tilde(text: &str, home: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (i, &c) in chars.iter().enumerate() {
        let starts_word =
            i == 0 || matches!(chars[i - 1], '\'' | '"') || chars[i - 1].is_whitespace();
        let ends_path = chars.get(i + 1).is_none_or(|&next| next == '/');
        if c == '~' && starts_word && ends_path {
            out.push_str(home);
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIR: &str = "/work/northwind";
    const HOME: &str = "/Users/demo";

    fn vars<'a>(url: Option<&'a str>, home: Option<&'a Path>) -> Vars<'a> {
        Vars {
            dir: Path::new(DIR),
            url,
            home,
        }
    }

    fn fill(text: &str) -> String {
        vars(Some("https://example.com"), Some(Path::new(HOME))).fill(text)
    }

    #[test]
    fn dir_is_replaced_by_the_project_folder() {
        assert_eq!(
            fill("cd ${dir} && ls ${dir}/src"),
            "cd /work/northwind && ls /work/northwind/src"
        );
    }

    #[test]
    fn url_is_replaced_when_the_project_has_one() {
        assert_eq!(fill("open ${url}/admin"), "open https://example.com/admin");
    }

    #[test]
    fn url_stays_literal_without_a_project_url() {
        let filled = vars(None, Some(Path::new(HOME))).fill("open ${url}");
        assert_eq!(filled, "open ${url}");
    }

    #[test]
    fn tilde_expands_where_it_starts_a_path() {
        for (input, expected) in [
            ("~", "/Users/demo"),
            ("~/x", "/Users/demo/x"),
            ("cd ~/x", "cd /Users/demo/x"),
            ("\"~/x\"", "\"/Users/demo/x\""),
            ("'~/x'", "'/Users/demo/x'"),
            ("cp ~/a ~/b", "cp /Users/demo/a /Users/demo/b"),
        ] {
            assert_eq!(fill(input), expected, "{input}");
        }
    }

    #[test]
    fn tilde_inside_a_word_or_naming_a_user_stays() {
        for input in [
            "a~/x",
            "~user",
            "~user/x",
            "https://example.com/~demo",
            "x~",
        ] {
            assert_eq!(fill(input), input, "{input}");
        }
    }

    #[test]
    fn tilde_stays_without_a_home_folder() {
        assert_eq!(vars(None, None).fill("cd ~/x"), "cd ~/x");
    }

    #[test]
    fn dir_and_tilde_combine() {
        assert_eq!(
            fill("${dir} ~/.cache"),
            "/work/northwind /Users/demo/.cache"
        );
    }
}
