use clap::Parser;
use colored::Colorize;
use search_and_replace::{load_configs_from_yaml, Config, SearchAndReplace};
use std::path::{Path, PathBuf};
use std::process;

#[derive(Parser, Debug)]
#[command(name = "search-and-replace")]
#[command(about = "A pre-commit hook that searches for strings and replaces them.")]
struct Cli {
    /// YAML config for multiple search and, optionally, replace operations.
    /// Default: .pre-commit-search-and-replace.yaml
    #[arg(short = 'c', long = "config", default_value = ".pre-commit-search-and-replace.yaml")]
    config: String,

    /// Search string or regexp (required if not using config)
    #[arg(short = 's', long = "search")]
    search: Option<String>,

    /// Replacement string
    #[arg(short = 'r', long = "replacement")]
    replacement: Option<String>,

    /// Case-insensitive search
    #[arg(short = 'i', long = "insensitive", default_value_t = false)]
    insensitive: bool,

    /// Extended regexp search
    #[arg(short = 'e', long = "extended", default_value_t = false)]
    extended: bool,

    /// Output coloring (default true, use --no-color to disable)
    #[arg(short = 'C', long = "color", default_value_t = true, action = clap::ArgAction::Set)]
    color: bool,

    /// Disable output coloring
    #[arg(long = "no-color")]
    no_color: bool,

    /// Write replacements to file (default true, use --no-write to disable)
    #[arg(short = 'w', long = "write", default_value_t = true, action = clap::ArgAction::Set)]
    write: bool,

    /// Disable writing replacements to file
    #[arg(long = "no-write")]
    no_write: bool,

    /// Files to search
    #[arg(trailing_var_arg = true, required = true)]
    files: Vec<String>,
}

fn main() {
    let cli = Cli::parse();

    // Determine color setting: --no-color flag, -C false, or NO_COLOR env var
    let use_color = cli.color && !cli.no_color && std::env::var("NO_COLOR").is_err();
    if !use_color {
        colored::control::set_override(false);
    }

    let write_enabled = cli.write && !cli.no_write;

    if cli.files.is_empty() {
        eprintln!("No files to search supplied as arguments!");
        process::exit(1);
    }

    // Filter out config file from the file list
    let config_path = Path::new(&cli.config);
    let files: Vec<PathBuf> = cli
        .files
        .iter()
        .map(PathBuf::from)
        .filter(|f| {
            std::fs::canonicalize(f).ok()
                != std::fs::canonicalize(config_path).ok()
        })
        .collect();

    // Load configs
    let configs: Vec<Config> = if let Some(ref search) = cli.search {
        vec![Config {
            search: search.clone(),
            replacement: cli.replacement.clone(),
            insensitive: Some(cli.insensitive),
            extended: Some(cli.extended),
            description: None,
        }]
    } else {
        match load_configs_from_yaml(config_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{} and no search argument specified.", e);
                process::exit(1);
            }
        }
    };

    let mut files_fixed: Vec<String> = Vec::new();
    let mut exit_status = 0;

    for (i, entry) in configs.iter().enumerate() {
        if entry.search.is_empty() {
            eprintln!("Config entry {} is missing required search string.", i + 1);
            process::exit(1);
        }

        let sar = SearchAndReplace::from_config(files.clone(), entry);
        let results = sar.parse_files();

        for result in &results {
            if result.is_empty() {
                // Clean up temp file if no occurrences
                if let Some(ref path) = result.replacement_file_path {
                    let _ = std::fs::remove_file(path);
                }
                continue;
            }

            let description = entry
                .description
                .as_deref()
                .unwrap_or(&entry.search);

            println!(
                "==== Found {} occurrences of \"{}\" in {}:\n",
                result.len().to_string().red(),
                description.yellow(),
                result.occurrences[0]
                    .file
                    .display()
                    .to_string()
                    .cyan()
            );

            for occ in &result.occurrences {
                println!("{}", occ);
            }
            println!();

            exit_status = 1;

            if entry.replacement.is_none() {
                if let Some(ref path) = result.replacement_file_path {
                    let _ = std::fs::remove_file(path);
                }
                continue;
            }

            if write_enabled {
                if let Some(ref src) = result.replacement_file_path {
                    let dest = &result.filename;
                    let dest_str = dest.display().to_string();

                    // Preserve permissions on Unix
                    #[cfg(unix)]
                    {
                        if let Ok(metadata) = std::fs::metadata(dest) {
                            let permissions = metadata.permissions();
                            let _ = std::fs::set_permissions(src, permissions);

                            // Try to preserve ownership (requires root)
                            use std::os::unix::fs::MetadataExt;
                            let uid = metadata.uid();
                            let gid = metadata.gid();
                            unsafe {
                                let c_path = std::ffi::CString::new(
                                    src.to_string_lossy().as_bytes(),
                                )
                                .unwrap();
                                libc::chown(c_path.as_ptr(), uid, gid);
                            }
                        }
                    }

                    std::fs::copy(src, dest).expect("Failed to copy replacement file");
                    let _ = std::fs::remove_file(src);
                    files_fixed.push(dest_str);
                }
            } else {
                // Clean up temp file when --no-write
                if let Some(ref path) = result.replacement_file_path {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }

    files_fixed.sort();
    files_fixed.dedup();
    for f in &files_fixed {
        println!("{}", format!("Fixed {}", f).green());
    }

    process::exit(exit_status);
}
