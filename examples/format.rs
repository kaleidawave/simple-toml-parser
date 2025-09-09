use simple_toml_parser::formatting::{FormatOptions, KeyIndentation, format_toml};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arg = std::env::args().nth(1);

    let source = if let Some("--content") = arg.as_deref() {
        std::env::args().nth(2).expect("no content")
    } else if let Some(path) = arg {
        std::fs::read_to_string(path).unwrap()
    } else {
        todo!()
    };

    // Copy the same configuration as the source
    let crlf = source
        .split_once('\n')
        .is_some_and(|(before, _)| before.ends_with("\r"));

    let options = FormatOptions {
        indentation: KeyIndentation::WholeDocument,
        prefix_indent_keys: false,
        value_indentation: "\t".into(),
        crlf,
    };

    let out = format_toml(&source, &options)?;

    eprintln!("--- output ---");
    eprintln!("{out}");

    Ok(())
}
