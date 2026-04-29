use colored::Colorize;
use regex::Regex;
use serde::Deserialize;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// Configuration entry from YAML or CLI args.
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub search: String,
    pub replacement: Option<String>,
    pub insensitive: Option<bool>,
    pub extended: Option<bool>,
    pub description: Option<String>,
}

/// Represents either a literal string search or a compiled regex.
#[derive(Debug, Clone)]
pub enum SearchPattern {
    Literal(String),
    Regex(Regex),
}

impl fmt::Display for SearchPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchPattern::Literal(s) => write!(f, "{}", s),
            SearchPattern::Regex(r) => write!(f, "{}", r.as_str()),
        }
    }
}

/// Core search-and-replace engine.
pub struct SearchAndReplace {
    pub search: SearchPattern,
    pub replacement: Option<String>,
    files: Vec<PathBuf>,
    insensitive: bool,
}

impl SearchAndReplace {
    pub fn new(
        files: Vec<PathBuf>,
        search: &str,
        insensitive: bool,
        extended: bool,
        replacement: Option<String>,
    ) -> Self {
        let pattern = parse_pattern(search, insensitive, extended);
        SearchAndReplace {
            search: pattern,
            replacement,
            files,
            insensitive,
        }
    }

    pub fn from_config(files: Vec<PathBuf>, config: &Config) -> Self {
        let insensitive = config.insensitive.unwrap_or(false);
        let extended = config.extended.unwrap_or(false);
        Self::new(
            files,
            &config.search,
            insensitive,
            extended,
            config.replacement.clone(),
        )
    }

    pub fn parse_files(&self) -> Vec<FileMatches> {
        self.files.iter().map(|f| self.parse_file(f)).collect()
    }

    fn parse_file(&self, filename: &Path) -> FileMatches {
        let file = fs::File::open(filename).expect(&format!("Unable to open file: {:?}", filename));
        let reader = BufReader::new(file);
        let mut all_occurrences = Vec::new();
        let mut temp_file: Option<NamedTempFile> = if self.replacement.is_some() {
            Some(NamedTempFile::new().expect("Failed to create temp file"))
        } else {
            None
        };

        for (lineno, line_result) in reader.lines().enumerate() {
            let line = line_result.expect("Failed to read line");
            let line_with_newline = format!("{}\n", line);
            let occurrences = self.search_line(filename, lineno + 1, &line_with_newline);

            if let Some(ref mut tf) = temp_file {
                if occurrences.is_empty() {
                    tf.write_all(line_with_newline.as_bytes())
                        .expect("Failed to write to temp file");
                } else {
                    let replaced = self.replace_line(&line_with_newline);
                    tf.write_all(replaced.as_bytes())
                        .expect("Failed to write to temp file");
                }
            }

            all_occurrences.extend(occurrences);
        }

        let temp_path = temp_file.map(|tf| {
            let (_, path) = tf.keep().expect("Failed to persist temp file");
            path
        });

        FileMatches {
            filename: filename.to_path_buf(),
            occurrences: all_occurrences,
            replacement_file_path: temp_path,
        }
    }

    fn replace_line(&self, line: &str) -> String {
        match (&self.search, &self.replacement) {
            (SearchPattern::Literal(s), Some(repl)) => {
                if self.insensitive {
                    case_insensitive_replace(line, s, repl)
                } else {
                    line.replace(s.as_str(), repl)
                }
            }
            (SearchPattern::Regex(re), Some(repl)) => {
                // Convert Ruby-style backreferences to regex crate style
                let rust_repl = ruby_replacement_to_rust(repl);
                re.replace_all(line, rust_repl.as_str()).to_string()
            }
            _ => line.to_string(),
        }
    }

    fn search_line(&self, filename: &Path, lineno: usize, line: &str) -> Vec<Occurrence> {
        // Check for no-search-replace comments
        if has_no_search_replace_comment(line) {
            return Vec::new();
        }

        let mut occurrences = Vec::new();
        let mut offset = 0;

        loop {
            match &self.search {
                SearchPattern::Literal(s) => {
                    let haystack = if self.insensitive {
                        line.to_lowercase()
                    } else {
                        line.to_string()
                    };
                    let needle = if self.insensitive {
                        s.to_lowercase()
                    } else {
                        s.clone()
                    };

                    match haystack[offset..].find(&needle) {
                        Some(pos) => {
                            let abs_pos = offset + pos;
                            // Check if replacement would change the line
                            if let Some(repl) = &self.replacement {
                                let replaced = if self.insensitive {
                                    case_insensitive_replace(line, s, repl)
                                } else {
                                    line.replace(s.as_str(), repl)
                                };
                                if replaced == line {
                                    break;
                                }
                            }
                            occurrences.push(Occurrence {
                                file: filename.to_path_buf(),
                                lineno,
                                col: abs_pos + 1,
                                length: s.len(),
                                context: line.to_string(),
                                replacement: self.replacement.clone(),
                            });
                            offset = abs_pos + 2;
                            if offset >= line.len() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
                SearchPattern::Regex(re) => {
                    match re.captures(&line[offset..]) {
                        Some(caps) => {
                            let m = caps.get(0).unwrap();
                            let abs_start = offset + m.start();

                            // Resolve backreferences in replacement
                            let actual_replacement = self.replacement.as_ref().map(|repl| {
                                resolve_backreferences(repl, &caps)
                            });

                            // Check if replacement would change the line
                            if let Some(ref _repl) = self.replacement {
                                let rust_repl = ruby_replacement_to_rust(_repl);
                                let replaced =
                                    re.replace_all(line, rust_repl.as_str()).to_string();
                                if replaced == line {
                                    break;
                                }
                            }

                            occurrences.push(Occurrence {
                                file: filename.to_path_buf(),
                                lineno,
                                col: abs_start + 1,
                                length: m.len(),
                                context: line.to_string(),
                                replacement: actual_replacement,
                            });
                            offset = abs_start + 2;
                            if offset >= line.len() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        occurrences
    }
}

/// A collection of occurrences found in a single file.
pub struct FileMatches {
    pub filename: PathBuf,
    pub occurrences: Vec<Occurrence>,
    pub replacement_file_path: Option<PathBuf>,
}

impl FileMatches {
    pub fn is_empty(&self) -> bool {
        self.occurrences.is_empty()
    }

    pub fn len(&self) -> usize {
        self.occurrences.len()
    }
}

/// A single match occurrence within a file.
pub struct Occurrence {
    pub file: PathBuf,
    pub lineno: usize,
    pub col: usize,
    pub length: usize,
    pub context: String,
    pub replacement: Option<String>,
}

impl fmt::Display for Occurrence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file_display = self.file.display().to_string().cyan();
        let context_clean = self.context.replace('\t', " ");
        let context_clean = context_clean.trim_end_matches('\n');
        let carets = "^".repeat(self.length).red();
        let padding = " ".repeat(self.col - 1);

        write!(
            f,
            "{}, line {}, col {}:\n    {}\n    {}{}",
            file_display, self.lineno, self.col, context_clean, padding, carets
        )?;

        if let Some(ref repl) = self.replacement {
            let mut chars: Vec<String> = self
                .context
                .trim_end_matches('\n')
                .chars()
                .map(|c| c.to_string())
                .collect();

            // Remove matched characters
            let start = self.col - 1;
            let end = (start + self.length).min(chars.len());
            for _ in start..end {
                if start < chars.len() {
                    chars.remove(start);
                }
            }
            // Insert colored replacement
            let colored_repl = repl.green().to_string();
            chars.insert(start, colored_repl);

            let replaced_line: String = chars.join("");
            let replaced_line = replaced_line.replace('\t', " ");

            write!(f, "\nAfter replacement:\n    {}", replaced_line)?;
        }

        Ok(())
    }
}

/// Detect if a string is in `/regexp/` format and compile accordingly.
pub fn parse_pattern(search: &str, insensitive: bool, extended: bool) -> SearchPattern {
    let re_detect = Regex::new(r"^/(.*)/$").unwrap();
    if let Some(caps) = re_detect.captures(search) {
        let pattern_str = &caps[1];
        let mut builder = regex::RegexBuilder::new(pattern_str);
        if insensitive {
            builder.case_insensitive(true);
        }
        if extended {
            builder.ignore_whitespace(true);
        }
        let re = builder
            .build()
            .unwrap_or_else(|e| panic!("Invalid regex '{}': {}", pattern_str, e));
        SearchPattern::Regex(re)
    } else {
        SearchPattern::Literal(search.to_string())
    }
}

/// Check if a line contains a no-search-replace comment marker.
fn has_no_search_replace_comment(line: &str) -> bool {
    let re = Regex::new(r"(//|/\*|#|<!--)\s*no-search-replace").unwrap();
    re.is_match(line)
}

/// Convert Ruby-style backreferences (\1, \2, \k<name>) to Rust regex style ($1, $2, ${name}).
fn ruby_replacement_to_rust(replacement: &str) -> String {
    let mut result = replacement.to_string();

    // Replace named backreferences: \k<name> -> ${name}
    let named_re = Regex::new(r"\\k<([^>]+)>").unwrap();
    result = named_re
        .replace_all(&result, |caps: &regex::Captures| {
            format!("${{{}}}", &caps[1])
        })
        .to_string();

    // Replace numbered backreferences: \1 -> $1, \2 -> $2, etc.
    let numbered_re = Regex::new(r"\\(\d+)").unwrap();
    result = numbered_re
        .replace_all(&result, |caps: &regex::Captures| {
            format!("${}", &caps[1])
        })
        .to_string();

    result
}

/// Resolve backreferences manually for display in Occurrence output.
fn resolve_backreferences(replacement: &str, caps: &regex::Captures) -> String {
    let mut result = replacement.to_string();

    // Replace numbered backreferences: \1, \2, etc.
    for i in 1..caps.len() {
        if let Some(m) = caps.get(i) {
            result = result.replace(&format!("\\{}", i), m.as_str());
        }
    }

    // Replace named backreferences: \k<name>
    let named_re = Regex::new(r"\\k<([^>]+)>").unwrap();
    let names: Vec<String> = named_re
        .captures_iter(&replacement)
        .map(|c| c[1].to_string())
        .collect();
    for name in names {
        if let Some(m) = caps.name(&name) {
            result = result.replace(&format!("\\k<{}>", name), m.as_str());
        }
    }

    result
}

/// Case-insensitive string replacement.
fn case_insensitive_replace(line: &str, search: &str, replacement: &str) -> String {
    let re = Regex::new(&format!("(?i){}", regex::escape(search))).unwrap();
    re.replace_all(line, replacement).to_string()
}

/// Load configs from a YAML file.
pub fn load_configs_from_yaml(path: &Path) -> Result<Vec<Config>, String> {
    let content = fs::read_to_string(path)
        .map_err(|_| format!("Unable to open {}", path.display()))?;
    let configs: Vec<Config> =
        serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse YAML: {}", e))?;
    Ok(configs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pattern_literal() {
        let p = parse_pattern("foobar", false, false);
        assert!(matches!(p, SearchPattern::Literal(s) if s == "foobar"));
    }

    #[test]
    fn test_parse_pattern_regex() {
        let p = parse_pattern("/Bad\\s*Regexp/", false, false);
        assert!(matches!(p, SearchPattern::Regex(_)));
    }

    #[test]
    fn test_parse_pattern_regex_insensitive() {
        let p = parse_pattern("/InsensitiveREGEXP/", true, false);
        match p {
            SearchPattern::Regex(re) => {
                assert!(re.is_match("insensitiveregexp"));
                assert!(re.is_match("INSENSITIVEREGEXP"));
            }
            _ => panic!("Expected regex"),
        }
    }

    #[test]
    fn test_ruby_replacement_to_rust() {
        assert_eq!(ruby_replacement_to_rust(r"\1 \2"), "$1 $2");
        assert_eq!(ruby_replacement_to_rust(r"\k<name>"), "${name}");
        assert_eq!(ruby_replacement_to_rust(r"\1\k<foo> \2"), "$1${foo} $2");
    }

    #[test]
    fn test_has_no_search_replace_comment() {
        assert!(has_no_search_replace_comment("something # no-search-replace"));
        assert!(has_no_search_replace_comment("something // no-search-replace"));
        assert!(has_no_search_replace_comment("something /* no-search-replace */"));
        assert!(has_no_search_replace_comment("something <!-- no-search-replace -->"));
        assert!(!has_no_search_replace_comment("normal line"));
    }

    #[test]
    fn test_resolve_backreferences_numbered() {
        let re = Regex::new(r"(\w+)\s+(\w+)").unwrap();
        let caps = re.captures("hello world").unwrap();
        let result = resolve_backreferences(r"\1-\2", &caps);
        assert_eq!(result, "hello-world");
    }

    #[test]
    fn test_resolve_backreferences_named() {
        let re = Regex::new(r"(?P<first>\w+)\s+(?P<second>\w+)").unwrap();
        let caps = re.captures("hello world").unwrap();
        let result = resolve_backreferences(r"\k<first>-\k<second>", &caps);
        assert_eq!(result, "hello-world");
    }
}
