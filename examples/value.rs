use simple_toml_parser::value;

fn main() {
    let values = &[
        r#"{ first = "Tom", last = "Preston-Werner" }"#,
        r#"{ x = 1, y = 2 }"#,
        r#"{ type.name = "pug", age = 78 }"#,
        r#"[ { x = 1, y = 2, z = 3 },
           { x = 7, y = 8, z = 9 },
           { x = 2, y = 4, z = 8 } ]"#,
    ];

    for value in values {
        eprintln!("{value}");
        let result = value::parse_with_exit_signal(value, |chain, _, value| {
            eprintln!("{chain:?} {value:?}");
            false
        });
        eprintln!("{result:?}");
    }
}
