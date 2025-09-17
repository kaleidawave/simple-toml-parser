use simple_toml_parser::TOMLParseError;
use simple_toml_parser::formatting::{FormatOptions, format_toml};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let root = args.next().expect("no entry point");

    let mut dry_run = false;
    let mut check = false;
    match args.next().as_deref() {
        Some("--dry-run") => {
            dry_run = true;
        }
        Some("--check") => {
            check = true;
        }
        Some(flag) => {
            eprintln!("Unknown flag '{flag}'");
            return Ok(());
        }
        None => {}
    }

    let mut found_unformatted_or_invalid_toml = false;

    if root == "cargo" {
        let root = std::env::current_dir().unwrap();
        eprintln!(
            "Formatting 'Cargo.toml's under {path}",
            path = root.display()
        );
        found_unformatted_or_invalid_toml = find_cargo_tomls(&root, &mut |path| {
            let result = process(path, dry_run, check);
            match result {
                Ok(valid) => !valid,
                Err(err) => {
                    eprintln!("{path} - {err:?}", path = path.display());
                    true
                }
            }
        })?;
    } else {
        for entry in glob::glob(&root).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    let valid = process(&path, dry_run, check)?;
                    if !valid {
                        found_unformatted_or_invalid_toml = true;
                    }
                }
                Err(e) => eprintln!("path error: {e:?}"),
            }
        }
    }

    if found_unformatted_or_invalid_toml {
        Err(Box::from("Found unformatted '.toml's"))
    } else {
        Ok(())
    }
}

fn process(path: &std::path::Path, dry_run: bool, check: bool) -> Result<bool, TOMLParseError> {
    let content = std::fs::read_to_string(&path).expect("Could not read file");
    let formatted = format_toml_content(&content)?;
    if dry_run {
        println!("{formatted}");
        Ok(true)
    } else {
        if content != formatted {
            if check {
                let difference = pretty_assertions::StrComparison::new(&formatted, &content);
                eprintln!("{path}\n{difference}", path = path.display());
                return Ok(false);
            } else {
                eprintln!("Modified {path}", path = path.display());
                std::fs::write(path, formatted).expect("failed to write to file");
            }
        } else {
            eprintln!("Matched");
        }
        Ok(true)
    }
}

fn format_toml_content(content: &str) -> Result<String, TOMLParseError> {
    // Copy the same configuration as the source
    let use_crlf = content
        .split_once('\n')
        .is_some_and(|(before, _)| before.ends_with("\r"));

    // TODO customise
    let options = FormatOptions {
        use_crlf,
        ..FormatOptions::default()
    };

    format_toml(content, &options)
}

fn find_cargo_tomls(
    root: &std::path::Path,
    cb: &impl Fn(&std::path::Path) -> bool,
) -> std::io::Result<bool> {
    let mut acc = false;
    if root.is_dir() {
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.file_name().is_some_and(|name| name == "Cargo.toml") {
                let result = cb(&path);
                acc |= result;
            }
            if path.is_dir()
                && path.file_name().is_some_and(|name| {
                    !matches!(
                        name.to_str().unwrap_or_default(),
                        "target" | ".git" | "node_modules" | "private"
                    )
                })
            {
                find_cargo_tomls(&path, cb)?;
            }
        }
    }
    Ok(acc)
}
