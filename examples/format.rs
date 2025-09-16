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

    for entry in glob::glob(&root).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let content = std::fs::read_to_string(&path).expect("Could not read file");
                let formatted = format_toml_content(&content)?;
                if dry_run {
                    println!("{formatted}");
                } else if check {
                    pretty_assertions::assert_eq!(formatted, content);
                    eprintln!("Matched");
                } else if content != formatted {
                    eprintln!("Modified {path}", path = path.display());
                    std::fs::write(path, formatted).expect("failed to write to file");
                }
            }
            Err(e) => eprintln!("path error: {e:?}"),
        }
    }

    Ok(())
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
