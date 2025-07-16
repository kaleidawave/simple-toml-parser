### Title

```toml
title = "TOML Example"
```

```
"title" => String("TOML Example")
```

### Table

```toml
[owner]
name = "Tom Preston-Werner"
```

```
["owner"] "name" => String("Tom Preston-Werner")
```

### Tables and inline tables

```toml
[database]
enabled = true
ports = [8000, 8001, 8002]
data = [["delta", "phi"], [3.14]]
temp_targets = { cpu = 79.5, case = 72.0 }
```

```
["database"] "enabled" => Boolean(true)
["database"] "ports" {0} => Number("8000")
["database"] "ports" {1} => Number("8001")
["database"] "ports" {2} => Number("8002")
["database"] "data" {0} {0} => String("delta")
["database"] "data" {0} {1} => String("phi")
["database"] "data" {1} {0} => Number("3.14")
["database"] "temp_targets" {"cpu"} => Number("79.5")
["database"] "temp_targets" {"case"} => Number("72.0")
```

### Dot 

```toml
[servers.alpha]
ip = "10.0.0.1"
role = "frontend"
```

```
["servers"."alpha"] "ip" => String("10.0.0.1")
["servers"."alpha"] "role" => String("frontend")
```

### Empty keys

```toml
= "no key name"  # INVALID
"" = "blank"     # VALID but discouraged
'' = 'blank'     # VALID but discouraged
```

```
Error: TOMLParseError { at: 0, reason: ExpectedKey }
```

### Dotted keys

```toml
name = "Orange"
physical.color = "orange"
physical.shape = "round"
site."google.com" = true
```

```
"name" => String("Orange")
"physical"."color" => String("orange")
"physical"."shape" => String("round")
"site"."google.com" => Boolean(true)
```

### Whitespace in keys

```toml
fruit.name = "banana"     # this is best practice
fruit. color = "yellow"    # same as fruit.color
fruit . flavor = "banana"   # same as fruit.flavor
```

```
"fruit"."name" => String("banana")
"fruit"."color" => String("yellow")
"fruit"."flavor" => String("banana")
```

### String keys

```toml
str = "I'm a string. \"You can quote me\". Name\tJos\u00E9\nLocation\tSF."
```

```
"str" => String("I'm a string. \"You can quote me\". Name\tJosé\nLocation\tSF.")
```

## Strings

### Multiline strings

```toml
str = """
Roses are red
Violets are blue"""
```

```
"str" => String("Roses are red\nViolets are blue")
```

### Multiline strings (with new line stuff)

```toml
str1 = "The quick brown fox jumps over the lazy dog."

str2 = """
The quick brown \


  fox jumps over \
    the lazy dog."""

str3 = """\
       The quick brown \
       fox jumps over \
       the lazy dog.\
       """
```

```
"str1" => String("The quick brown fox jumps over the lazy dog.")
"str2" => String("The quick brown fox jumps over the lazy dog.")
"str3" => String("The quick brown fox jumps over the lazy dog.")
```

### Quotation marks

```toml
str4 = """Here are two quotation marks: "". Simple enough."""
# str5 = """Here are three quotation marks: """."""  # INVALID
str5 = """Here are three quotation marks: ""\"."""
str6 = """Here are fifteen quotation marks: ""\"""\"""\"""\"""\"."""

# "This," she said, "is just a pointless statement."
str7 = """"This," she said, "is just a pointless statement.""""
```

```
"str4" => String("Here are two quotation marks: \"\". Simple enough.")
"str5" => String("Here are three quotation marks: \"\"\".")
"str6" => String("Here are fifteen quotation marks: \"\"\"\"\"\"\"\"\"\"\"\"\"\"\".")
"str7" => String("\"This,\" she said, \"is just a pointless statement.\"")
```

### Literal strings

```toml
winpath  = 'C:\Users\nodejs\templates'
winpath2 = '\\ServerX\admin$\system32\'
quoted   = 'Tom "Dubs" Preston-Werner'
regex    = '<\i\c*\s*>'
```

```
"winpath" => String('C:\Users\nodejs\templates')
"winpath2" => String('\\ServerX\admin$\system32\')
"quoted" => String('Tom "Dubs" Preston-Werner')
"regex" => String('<\i\c*\s*>')
```

### Literal strings (multiline)

```toml
str1 = '''I [dw]on't need \d{2} apples'''
str2 = '''
The first newline is
trimmed in raw strings.
   All other whitespace
   is preserved.
'''
```

```
"str1" => String('I [dw]on't need \d{2} apples')
"str2" => String('The first newline is\ntrimmed in raw strings.\n   All other whitespace\n   is preserved.\n')
```

## Tables

### Nested tables

```toml
[dog."tater.man"]
type.name = "pug"
```

```
["dog"."tater.man"] "type"."name" => String("pug")
```
