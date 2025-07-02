### Title

```toml
title = "TOML Example"
```

```
[Slice("title", Alone)] -> String("TOML Example")
```

### Table

```toml
[owner]
name = "Tom Preston-Werner"
```

```
[Slice("owner", Alone), Slice("name", Alone)] -> String("Tom Preston-Werner")
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
[Slice("database", Alone), Slice("enabled", Alone)] -> Boolean(true)
[Slice("database", Alone), Slice("ports", Alone), Index(0)] -> Number("8000")
[Slice("database", Alone), Slice("ports", Alone), Index(1)] -> Number("8001")
[Slice("database", Alone), Slice("ports", Alone), Index(2)] -> Number("8002")
[Slice("database", Alone), Slice("data", Alone), Index(0), Index(0)] -> String("delta")
[Slice("database", Alone), Slice("data", Alone), Index(0), Index(1)] -> String("phi")
[Slice("database", Alone), Slice("data", Alone), Index(1), Index(0)] -> Number("3.14")
[Slice("database", Alone), Slice("temp_targets", Alone), Slice("cpu", Alone)] -> Number("79.5")
[Slice("database", Alone), Slice("temp_targets", Alone), Slice("case", Alone)] -> Number("72.0")
```

### Dot 

```toml
[servers.alpha]
ip = "10.0.0.1"
role = "frontend"
```

```
[Slice("servers", Alone), Slice("alpha", Dot), Slice("ip", Alone)] -> String("10.0.0.1")
[Slice("servers", Alone), Slice("alpha", Dot), Slice("role", Alone)] -> String("frontend")
```

### Empty keys

```toml
= "no key name"  # INVALID
"" = "blank"     # VALID but discouraged
'' = 'blank'     # VALID but discouraged
```

```
...
```

### Dotted keys

```toml
name = "Orange"
physical.color = "orange"
physical.shape = "round"
site."google.com" = true
```

```
[Slice("name", Alone)] -> String("Orange")
[Slice("physical", Alone), Slice("color", Dot)] -> String("orange")
[Slice("physical", Alone), Slice("shape", Dot)] -> String("round")
[Slice("site", Alone), Slice("google.com", Dot)] -> Boolean(true)
```

### Whitespace in keys

```toml
fruit.name = "banana"     # this is best practice
fruit. color = "yellow"    # same as fruit.color
fruit . flavor = "banana"   # same as fruit.flavor
```

```
[Slice("fruit", Alone), Slice("name", Dot)] -> String("banana")
[Slice("fruit", Alone), Slice("color", Dot)] -> String("yellow")
[Slice("fruit", Alone), Slice("flavor", Dot)] -> String("banana")
```

### String keys

```toml
str = "I'm a string. \"You can quote me\". Name\tJos\u00E9\nLocation\tSF."
```

```
...
```

## Strings

### Multiline strings

```toml
str1 = """
Roses are red
Violets are blue"""
```

```
...
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
...
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
...
```

### Literal strings

```toml
# What you see is what you get.
winpath  = 'C:\Users\nodejs\templates'
winpath2 = '\\ServerX\admin$\system32\'
quoted   = 'Tom "Dubs" Preston-Werner'
regex    = '<\i\c*\s*>'
```

```
...
```

### Literal strings (multiline)

```toml
regex2 = '''I [dw]on't need \d{2} apples'''
lines  = '''
The first newline is
trimmed in raw strings.
   All other whitespace
   is preserved.
'''
```

```
...
```

## Tables

### Nested tables

```toml
[dog."tater.man"]
type.name = "pug"
```

```
...
```
