use simple_toml_parser::parse as parse_toml;

fn main() {
    let example = r#"
# This is a TOML document

title = "TOML Example"

[owner]
name = "Tom Preston-Werner"
dob = 1979-05-27T07:32:00-08:00

[database]
enabled = true
ports = [8000, 8001, 8002]
data = [["delta", "phi"], [3.14]]
temp_targets = { cpu = 79.5, case = 72.0 }

[servers]

[servers.alpha]
ip = "10.0.0.1"
role = "frontend"

[servers.beta]
ip = "10.0.0.2"
"#
    .trim_start();

    let source = if let Some(path) = std::env::args().nth(1) {
        std::fs::read_to_string(path).unwrap()
    } else {
        example.to_owned()
    };

    // let second_arg = std::env::args().nth(2);
    // let mode = second_arg
    //     .and_then(|arg| {
    //         matches!(arg.as_str(), "--verbose" | "--text").then_some(arg[2..].to_owned())
    //     })
    //     .unwrap_or_default();

    parse_toml(&source, |keys, _, value| {
        eprintln!("{keys:?} -> {value:?}");
    })
    .unwrap();
}
