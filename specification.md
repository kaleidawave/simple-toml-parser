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

### Values

```toml
a = false
b = true
```

```
"a" => Boolean(false)
"b" => Boolean(true)
```

### Arrays

```toml
enabled = true
ports = [8000, 8001, 8002]
data = [["delta", "phi"], [3.14]]
```

```
"enabled" => Boolean(true)
"ports" {0} => Number(8000)
"ports" {1} => Number(8001)
"ports" {2} => Number(8002)
"data" {0} {0} => String("delta")
"data" {0} {1} => String("phi")
"data" {1} {0} => Number(3.14)
```

### Inline tables

```toml
temp_targets = { cpu = 79.5, case = 72.0 }
```

```
"temp_targets" {"cpu"} => Number(79.5)
"temp_targets" {"case"} => Number(72.0)
```

### Dotted table keys

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

### Dotted specifier keys

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

### Multiline strings (with new lines)

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
"str2" => String('The first newline is\ntrimmed in raw strings.\n All other whitespace\n\t is preserved.\n')
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

## Comments

> This is for formatting below

### Basic comment

```toml
yield comments
---
# this is a comment
```

```
 => Comment("this is a comment")
```

### On the same line

```toml
yield comments
---
x = 2 # this is a comment

array = [
	3 # comment after
]
```

```
"x" => Number(2)
"x" => Comment("this is a comment") after value
"array" {0} => Number(3)
"array" {0} => Comment("comment after") after value
```

## Formatting

### Format keys

> Whitespace of adjacent keys removed, whitespace around `=`

```toml
format
---
key1="hi"

key2 =  "value"
```

```toml
key1 = "hi"
key2 = "value"
```

### Format tables

> Each table has keys directly after with trailing new line

```toml
format
---
[table1]

key1 = 1
[table2]
key2 = 2
```

```toml
[table1]
key1 = 1

[table2]
key2 = 2
```

### Comments retained

> Comments have three modes: on line, annotating next line or standalone

```toml
format
---
[table]
key1 = "hi" # comment here

# comment about key2
key2 = "test"

# general comment

key3 = "hi"
```

```toml
[table]
key1 = "hi" # comment here

# comment about key2
key2 = "test"

# general comment

key3 = "hi"
```

### Formatting array literals

```toml
format
---
a = [1, 2,
3]
```

```toml
a = [
	1,
	2,
	3
]
```

### Formatting nested array literals

```toml
format
---
a = [[1, 2, 3], [4, 5,6]]
```

```toml
a = [
	[
		1,
		2,
		3
	],
	[
		4,
		5,
		6
	]
]
```

### Formatting inline table literals

```toml
format
---
it1 = { name = "value", 
version = "0.5.1" }
it2 = { name = "x", nested = {

data = "12323" }, version="1.6.2" }
```

```toml
it1 = { name = "value", version = "0.5.1" }
it2 = { name = "x", nested = { data = "12323" }, version = "1.6.2" }
```

### inline table of arrays

```toml
format
---
inline_table_array = { a = [1], b = [2, 3, 4], c = [5] }
```

```toml
inline_table_array = { a = [1], b = [2, 3, 4], c = [5] }
```

### Array of inline_tables

```toml
format
---
array_inline_table = [{a = 1}, { b = 2, c = 3, d = 4 }]
```

```toml
array_inline_table = [
	{ a = 1 },
	{ b = 2, c = 3, d = 4 }
]
```

### Deeply nested array

```toml
format
---
aaaaa = [[[[[1]]]]]
```

```toml
aaaaa = [
	[
		[
			[
				[1]
			]
		]
	]
]
```

### Deeply nested inline_table

```toml
format
---
itititit = { a = { b = { c = { d = { e = 1 } } } } }
```

```toml
itititit = { a = { b = { c = { d = { e = 1 } } } } }
```

### Comments in array

```toml
format
---
array = [1 # hi
]
```

```toml
array = [
	1 # hi
]
```

### Comments in inline-tables

```toml
format
---
it = { a = 1 # hi
}
```

> new lines in inline-tables are discouraged. So these comments currently get removed...

```toml
it = { a = 1 }
```

### Comments after table

```toml
format
---
[table] # table things
a = 1
```

```toml
[table] # table things
a = 1
```

### Discern between before and after

```toml
format
---
array = [1 
] 

# hi

a = [
	"b",
	# "c"
]
```

```toml
array = [1]

# hi

a = [
	"b"
	# "c"
]
```