use simple_toml_parser::formatting::{FormatOptions, Indentation, format_toml};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arg = std::env::args().nth(1);

    let source = if let Some("--content") = arg.as_deref() {
        std::env::args().nth(2).expect("no content")
    } else if let Some(path) = arg {
        std::fs::read_to_string(path).unwrap()
    } else {
        todo!()
    };

    let options = FormatOptions {
        indentation: Indentation::WholeDocument,
        prefix_indent_keys: false,
    };
    let out = format_toml(&source, &options)?;

    eprintln!("--- output ---");
    eprintln!("{out}");

    Ok(())
}
