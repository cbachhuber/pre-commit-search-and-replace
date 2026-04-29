use search_and_replace::{load_configs_from_yaml, Config, SearchAndReplace};
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn fixture_files() -> Vec<PathBuf> {
    vec![
        fixture_path("bad_content.txt"),
        fixture_path("good_content.txt"),
        fixture_path("ignore_content.txt"),
    ]
}

fn load_test_configs() -> Vec<Config> {
    load_configs_from_yaml(&fixture_path("search-and-replace.yaml")).unwrap()
}

fn sar_from_config(config: &Config) -> SearchAndReplace {
    SearchAndReplace::from_config(fixture_files(), config)
}

// ===== Config loading tests =====

#[test]
fn foobar_config_has_string_search() {
    let configs = load_test_configs();
    let sar = sar_from_config(&configs[0]); // foobar
    match &sar.search {
        search_and_replace::SearchPattern::Literal(s) => assert_eq!(s, "foobar"),
        _ => panic!("Expected literal search"),
    }
    assert_eq!(sar.replacement.as_deref(), Some("fooBAZ"));
}

#[test]
fn bad_regexp_config_has_regex_search() {
    let configs = load_test_configs();
    let sar = sar_from_config(&configs[1]); // bad regexp
    match &sar.search {
        search_and_replace::SearchPattern::Regex(re) => {
            assert_eq!(re.as_str(), r"Bad\s*Regexp");
        }
        _ => panic!("Expected regex search"),
    }
    assert!(sar.replacement.is_none());
}

#[test]
fn insensitive_regex_config() {
    let configs = load_test_configs();
    let sar = sar_from_config(&configs[2]); // insensitive regex
    match &sar.search {
        search_and_replace::SearchPattern::Regex(re) => {
            assert!(re.is_match("insensitiveregexp"));
            assert!(re.is_match("INSENSITIVEREGEXP"));
        }
        _ => panic!("Expected regex search"),
    }
    assert!(sar.replacement.is_none());
}

// ===== Occurrence count tests =====

#[test]
fn foobar_has_one_occurrence_in_bad_file() {
    let configs = load_test_configs();
    let sar = sar_from_config(&configs[0]);
    let results = sar.parse_files();
    assert_eq!(results[0].len(), 1); // bad_content.txt
    assert!(results[1].is_empty()); // good_content.txt
}

#[test]
fn bad_regexp_has_two_occurrences_in_bad_file() {
    let configs = load_test_configs();
    let sar = sar_from_config(&configs[1]);
    let results = sar.parse_files();
    assert_eq!(results[0].len(), 2); // bad_content.txt
    assert!(results[1].is_empty()); // good_content.txt
}

#[test]
fn insensitive_regex_has_one_occurrence_in_bad_file() {
    let configs = load_test_configs();
    let sar = sar_from_config(&configs[2]);
    let results = sar.parse_files();
    assert_eq!(results[0].len(), 1); // bad_content.txt
    assert!(results[1].is_empty()); // good_content.txt
}

#[test]
fn insensitive_string_has_one_occurrence_in_bad_file() {
    let configs = load_test_configs();
    let sar = sar_from_config(&configs[3]);
    let results = sar.parse_files();
    assert_eq!(results[0].len(), 1); // bad_content.txt
    assert!(results[1].is_empty()); // good_content.txt
}

#[test]
fn regex_foobar_no_occurrence_when_replacement_unchanged() {
    let configs = load_test_configs();
    let sar = sar_from_config(&configs[4]);
    let results = sar.parse_files();
    assert_eq!(results[0].len(), 0); // no-op replacement
}

// ===== Comment ignoring tests =====

#[test]
fn ignores_hash_comment_no_search_replace() {
    let configs = load_test_configs();
    let sar = sar_from_config(&configs[5]); // "some special text"
    let results = sar.parse_files();
    assert_eq!(results[2].len(), 0); // ignore_content.txt
}

#[test]
fn ignores_double_slash_comment_no_search_replace() {
    let configs = load_test_configs();
    let sar = sar_from_config(&configs[6]); // "some very special text"
    let results = sar.parse_files();
    assert_eq!(results[2].len(), 0); // ignore_content.txt
}

// ===== Backreference tests =====

#[test]
fn doxygen_backreference_replacement() {
    let configs = load_test_configs();
    let sar = sar_from_config(&configs[7]); // doxygen
    let results = sar.parse_files();
    assert_eq!(results[0].len(), 3); // bad_content.txt
    assert_eq!(
        results[0].occurrences[0].replacement.as_deref(),
        Some("/// @param[in] some_param_P1")
    );
}

#[test]
fn named_capture_backreference_replacement() {
    let configs = load_test_configs();
    let sar = sar_from_config(&configs[8]); // named capture
    let results = sar.parse_files();
    assert_eq!(results[0].len(), 1); // bad_content.txt
    assert_eq!(
        results[0].occurrences[0].replacement.as_deref(),
        Some("It is foobar")
    );
}

#[test]
fn capture_group_search_without_replacement() {
    let configs = load_test_configs();
    let sar = sar_from_config(&configs[9]); // capture group search
    let results = sar.parse_files();
    assert_eq!(results[0].len(), 1); // bad_content.txt
    assert!(results[1].is_empty()); // good_content.txt
}

// ===== Occurrence display test =====

#[test]
fn occurrence_display_format() {
    let configs = load_test_configs();
    let sar = sar_from_config(&configs[0]); // foobar
    let results = sar.parse_files();
    let occ = &results[0].occurrences[0];
    let display = format!("{}", occ);

    // Should contain file path, line number, col number
    assert!(display.contains("line 4"));
    assert!(display.contains("col 13"));
    assert!(display.contains("foobar"));
    assert!(display.contains("After replacement:"));
    assert!(display.contains("fooBAZ"));
}

// ===== CLI integration tests =====

#[cfg(test)]
mod cli_tests {
    use assert_cmd::Command;
    use std::fs;
    use tempfile::NamedTempFile;

    fn fixture_path(name: &str) -> String {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
            .display()
            .to_string()
    }

    fn copy_fixture_to_temp(name: &str) -> NamedTempFile {
        let src = fixture_path(name);
        let tmp = NamedTempFile::new().unwrap();
        fs::copy(&src, tmp.path()).unwrap();
        tmp
    }

    #[test]
    fn good_file_exits_normally() {
        let tmp = copy_fixture_to_temp("good_content.txt");
        Command::cargo_bin("search-and-replace")
            .unwrap()
            .args([
                "-s",
                "Something",
                "--no-color",
                tmp.path().to_str().unwrap(),
            ])
            .assert()
            .code(0);
    }

    #[test]
    fn bad_file_exits_with_error() {
        let tmp = copy_fixture_to_temp("bad_content.txt");
        Command::cargo_bin("search-and-replace")
            .unwrap()
            .args(["-s", "foobar", "--no-color", tmp.path().to_str().unwrap()])
            .assert()
            .code(1);
    }

    #[test]
    fn bad_file_insensitive_search_exits_with_error() {
        let tmp = copy_fixture_to_temp("bad_content.txt");
        Command::cargo_bin("search-and-replace")
            .unwrap()
            .args([
                "-s",
                "There are SO many",
                "-i",
                "--no-color",
                tmp.path().to_str().unwrap(),
            ])
            .assert()
            .code(1);
    }

    #[test]
    fn bad_file_with_replacement_writes_file() {
        let tmp = copy_fixture_to_temp("bad_content.txt");
        Command::cargo_bin("search-and-replace")
            .unwrap()
            .args([
                "-s",
                "foobar",
                "-r",
                "youbar",
                "--no-color",
                tmp.path().to_str().unwrap(),
            ])
            .assert()
            .code(1);

        let content = fs::read_to_string(tmp.path()).unwrap();
        assert!(!content.contains("foobar"));
        assert!(content.contains("youbar"));
    }

    #[test]
    fn bad_file_with_no_write_does_not_modify() {
        let tmp = copy_fixture_to_temp("bad_content.txt");
        Command::cargo_bin("search-and-replace")
            .unwrap()
            .args([
                "-s",
                "foobar",
                "-r",
                "youbar",
                "--no-write",
                "--no-color",
                tmp.path().to_str().unwrap(),
            ])
            .assert()
            .code(1);

        let content = fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("foobar"));
        assert!(!content.contains("youbar"));
    }

    #[test]
    fn config_file_applies_all_rules() {
        let tmp = copy_fixture_to_temp("bad_content.txt");
        Command::cargo_bin("search-and-replace")
            .unwrap()
            .args([
                "-c",
                &fixture_path("search-and-replace.yaml"),
                "--no-color",
                tmp.path().to_str().unwrap(),
            ])
            .assert()
            .code(1);

        let content = fs::read_to_string(tmp.path()).unwrap();
        assert!(!content.contains("@param  second"));
        assert!(!content.contains("\\1 "));
        assert!(content.contains("@param[in]"));
    }

    #[test]
    fn ignored_hash_comment_exits_normally() {
        let tmp = copy_fixture_to_temp("ignore_content.txt");
        Command::cargo_bin("search-and-replace")
            .unwrap()
            .args([
                "-s",
                "some special text",
                "--no-color",
                tmp.path().to_str().unwrap(),
            ])
            .assert()
            .code(0);
    }

    #[test]
    fn ignored_double_slash_comment_exits_normally() {
        let tmp = copy_fixture_to_temp("ignore_content.txt");
        Command::cargo_bin("search-and-replace")
            .unwrap()
            .args([
                "-s",
                "some very special text",
                "--no-color",
                tmp.path().to_str().unwrap(),
            ])
            .assert()
            .code(0);
    }

    #[test]
    fn ignored_c_comment_exits_normally() {
        let tmp = copy_fixture_to_temp("ignore_content.txt");
        Command::cargo_bin("search-and-replace")
            .unwrap()
            .args([
                "-s",
                "has a comment in the middle",
                "--no-color",
                tmp.path().to_str().unwrap(),
            ])
            .assert()
            .code(0);
    }

    #[test]
    fn ignored_xml_comment_exits_normally() {
        let tmp = copy_fixture_to_temp("ignore_content.txt");
        Command::cargo_bin("search-and-replace")
            .unwrap()
            .args([
                "-s",
                "an XML style comment",
                "--no-color",
                tmp.path().to_str().unwrap(),
            ])
            .assert()
            .code(0);
    }
}
