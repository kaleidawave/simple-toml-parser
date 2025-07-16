use simple_toml_parser::{
    parse_with_exit_signal as parse_toml, RootTOMLValue, TOMLKey, TOMLKeyContext,
};

static EXAMPLE: &str = r#"
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
"#;

fn main() {
    let arg = std::env::args().nth(1);

    if let Some("--interactive") = arg.as_deref() {
        run_interactive();
        return;
    }

    let source = if let Some("--content") = arg.as_deref() {
        std::env::args().nth(2).expect("no content")
    } else if let Some(path) = arg {
        std::fs::read_to_string(path).unwrap()
    } else {
        EXAMPLE.trim_start().to_owned()
    };

    // parse_toml(&source, |keys, context, _value| {
    //     println!("{keys:?} {context:?}");
    //     false
    // }).unwrap();

    parse_toml(&source, |keys, context, value| {
        debug_keys(keys, context);
        debug_value(value);
        false
    })
    .unwrap();
}

// TODO better formatting needed
fn debug_keys(keys: &[TOMLKey<'_>], context: &TOMLKeyContext) {
    fn debug_keys_(keys: &[TOMLKey<'_>]) {
        let mut first = false;
        for key in keys {
            if first {
                print!(".");
            }
            match key {
                TOMLKey::Slice(item) => print!("{item:?}"),
                TOMLKey::Index(item) => print!("{item:?}"),
            }
            first = true;
        }
    }

    let (table_keys, specifier_keys, object_keys) = context.split_keys(keys);

    if !table_keys.is_empty() {
        print!("[");
        debug_keys_(table_keys);
        print!("] ");
    }

    if !specifier_keys.is_empty() {
        debug_keys_(specifier_keys);
    }

    for keys in object_keys {
        print!(" {{");
        debug_keys_(keys);
        print!("}}");
    }

    print!(" => ")
}

fn debug_value(value: RootTOMLValue<'_>) {
    use std::borrow::Cow;

    if let RootTOMLValue::String(value) = value {
        if value.is_literal() {
            let mut start = 0;
            let value = value.raw();
            let mut escaped = Cow::Borrowed("");
            for (idx, matched) in value.match_indices(&['\t', '\r', '\n']) {
                escaped += Cow::Borrowed(&value[start..idx]);
                match matched {
                    "\t" => {
                        escaped += Cow::Borrowed("\\t");
                    }
                    "\n" => {
                        escaped += Cow::Borrowed("\\n");
                    }
                    "\r" => {
                        escaped += Cow::Borrowed("\\r");
                    }
                    chr => unreachable!("{chr}"),
                }
                start = idx + 1;
            }
            escaped += Cow::Borrowed(&value[start..]);
            println!("String('{escaped}')");
        } else {
            println!("String({value:?})", value = value.value());
        }
    } else {
        println!("{value:?}");
    }
}

fn run_interactive() {
    use std::io::{stdin, BufRead};
    let stdin = stdin();
    let mut buf = Vec::new();

    println!("start");

    for line in stdin.lock().lines().map_while(Result::ok) {
        if line == "close" {
            if !buf.is_empty() {
                eprintln!("no end to message {buf:?}");
            }
            break;
        }

        if line == "end" {
            let output = String::from_utf8_lossy(&buf);
            // println!("{output}");
            let out = parse_toml(&output, |keys, context, value| {
                debug_keys(keys, context);
                debug_value(value);
                false
            });
            if let Err(error) = out {
                println!("Error: {error:?}");
            }
            println!("end");
            buf.clear();
            continue;
        }

        buf.extend_from_slice(line.as_bytes());
        buf.push(b'\n');
    }
}
