# Weave Syntax

## Overview
Weave is a language for writing simple, data-pipeline-oriented shell scripts.

Weaver is a Rust-based interpreter for the Weave language.

## Goals

1. Clean syntax; Composable functionality
2. Pipelines of data transformations should be trivial to express and compose
3. First-class function support
4. Make common data types trivial to work with:
  - Plain text
  - CSV
  - JSON
  - INI
  - TOML
  - YAML

## Anti-Goals

1. Weave is not an all-purpose programming language. Many features common in “real” languages are out of scope such as Classes.
2. An extensible type system - see above. Weave's Containers are the idiomatic method of structuring data.
3. Windows support. Weave is targeted for POSIX environments. If it happens to work on Windows, that’s great, but it’s not promised nor supported.

# Syntax

## Basic Code

```weave
# Comments begin with a # and run to the end of a line
# Multi-line comments do not exist. Just keep adding new comment lines.

# assignment takes the form <identifier> = <rval>
a = 1
b = 2

# Numbers are 64-bit values. Under the hood, the binary representation will be u64, i64 or f64 
# depending on the needs of the program. Everything _wants_ to be a u64, but if you negate or divide
# we'll change the value as needed.

# Strings use double quotes
str = “This is a string”

# Symbols are immutable strings which begin with a :
# and have no spaces. Letters, numbers and underscores are allowed.
:asymbol
:also_a_symbol
:1 # technically a valid symbol, but you’re weird if you do this.

# Containers
# A Container is like both a List or a Dict/Map/Hashmap in other languages.

# By default, if you declare a Container with a list of values, it behaves like a List 
[ 1, 2, 3 ]

# Containers may contain multiple types - including more Containers
[ 1, :a, [ 27, a_fn ] ]

# To get hashmap-like behavior, values in a Container can be paired with a key.
c = [ a: 1, b: 2 ]

# Keys can then be used to access the values in the Container
c[:b] == 2  # true
c[:a] = 3   # and you can assign with them as well
```

## Math

```weave
# math uses standard infix format and operators
c = a + b # 3
d = a / b # 0.5 - conversion from int -> float happens automatically
e = b - a # 1 

# Addition
3.0 + “a”         # adding different types is not allowed, but...
“a” + "3" # “a3”    ...Strings can be concatenated with '+'
```

## Functions and Lambdas

```weave
# Functions begin with the ‘fn’ keyword
fn add(arg1, arg2) {
  # The last statement in a function is implicitly returned
  arg1 + arg2
}

# Function params may have default values using Pair syntax:
fn sum(acc: 0, values) { ... }
total = sum(numbers)
total_plus_one = sum(1, numbers)

# Lambdas are declared with a caret ^
l = ^(arg1, arg2) { arg1 + arg2 }

# lambdas may be multi-line.
l = ^(a, b) {
  a += 1
  b *= 2
  a * b
}

# lambdas go out of scope when their variable goes out of scope
fn adder(a, b) { a + b }

fn add_3(a) {
   # adder is 'in scope' here as a standard fn declaration
   adder(a, 3)
}

fn add_v2(a, b) {
  sum = ^(a, b) { a + b } # 'sum' is only present within this scope
  sum(a, b)
}

# invocation requires parens
total = add(a, b)

# you can pass functions to other functions by passing their identifier. 

fn partial(func, arg1) {
  # builds and returns a lambda fn - so long as someone has a reference to it
  # this lambda will be available!
  ^(*args) { func(arg1, *args) }
}

add_5 = partial(add, 5)

# Closures are a thing too!
fn outer(n) {
  a = 1
  inner = ^(b) { n + a + b }
  inner
}

four = 4
add_5 = outer(four)  # add 4 is a lambda fn which holds a closure over "four" and "a"
add_5(3) == 8        # 8 = b(arg: 3) + n(global.four) + a(outer.1)  

# Params can be invoked by name or position
fn div(a, b) {
  a/b
}

# Calling by name uses Pair syntax from Containers again!
div(b: 3, a: 2) # 0.6666...

sum_a_and_b(a: 3, b: 4) # 7
sum_a_and_b(b: 4, a: 3, c: 12) # Error, :c is not a valid param!
```

## Function Pipelines

Pipelines are one of the core features of Weave! These three operators take the place of virtually everything you’d use a standard loop for in other languages.

In Weave, repeated operations on Container items are handled with one of these operators:

-  |> pipe
-  *> map
-  &> reduce

### Pipe

The Pipe |> operator is used to pass data into a function, passed as an implicit argument.
```weave
data = [1, 2, 3]
fn display(c) { print(c) }

# This is equivalent to calling display(data)... but opens up some further options for us too - read on!
data |> display   
```

### Map

The Map *> operator transforms a Value or each item in a Container by invoking the following function on it.
```weave
data = [1, 2, 3]
fn dbl(x) { x * 2 }

# this calls dbl(x) on each value in data, then returns the resulting Container
doubled = data *> dbl   # [2, 4, 6]
```

### Reduce

The Reduce &> operator invokes the following function on each value in a Container, accumulating the result in a new value. The function must accept two values, the first of which (the accumulator value) must have a default value. This is the equivalent of a “left”-fold in other languages.
```weave
data = [1, 2, 3]
fn sum(acc: 0, v) { acc + v }

total = data &> sum   # 6
```

### Example

Function Pipelines are composable so that you can quickly combine a series of functions to transform your data however you need

```weave
# A list of lambdas we'll use to get the total of the "total" column in a set of CSV files

# All CSVs
csvs = ^() { `ls *.csv`.output.lines() }

# Open a file, then extract just 'total' values
totals_in = ^(filename) { read(open(filename, :read), :csv).map(^(x) { x[:total] }) }

# Sum a list of values 
sum = ^(acc: 0, val) { acc + val }

# Sum a list _of_ lists
sum_lists(vals) { vals.reduce(sum) }

# Define our pipeline. We're combining shell calls, iterators and so forth, but each
# output is simply forwarded to the next function.

total = csvs 
          *> totals_in  # Map to a list of lists of values
          *> sum_arr    # sum each list to a single value
          &> sum        # sum the final list
          |> ^(total) { print("Total: {total}") }  # print with an in-line lambda
```

## Containers

```weave
# Container objects are Lists and/or Hash maps - depending on how you look at them.

# Lists are lists of values.
# Maps are Lists of Pairs.

# A Pair is a tuple of a symbol followed by a Value
a: 1  # This is a Pair of :a and 1.

# A Container may be indexed using either a symbol or an integer. 
# Values are sorted and indexed by insertion order
m = [ a: 1, b: 3, c: 5 ]  # Map-ish Container
l = [ 1, 3, 5 ]           # List-ish Container

# Integer indexing is by value order:
m[0] # yields 1
l[0] # yields 1

# Symbol indexing only works if the key exists
m[:b] # yields 3
l[:b] # ERR - no such key ':b'

# Iterating over lists or maps yields their values in insertion order
m |> ^(x) { print(x) }  # prints 1, 3, 5
l |> ^(x) { print(x) }  # prints 1, 3, 5

# keys can be retrieved
m.keys # [ :a, :b, :c ] - insertion order

# But you don't have to have any
l.keys # [] 

# Containers may be sorted - which will rearrange keys to match
m.sort(:desc)  # sorts the values to [c: 5, b: 3, a: 1]
m.keys # [ :c, :b, :a ]

# Keys may be added to a Container that does not already have them
l[:x] = 12 # l is now [1, 2, 3, x: 12]
l.keys     # [:x]
l[:x]      # 12

# Bare values can be added to Containers which have keys
m << "bare value"  # [a: 1, b: 3, c: 5, "bare value"]

# Containers provide Set functions on their values
# Union:
[ 1, 2, 3 ] | [ 2, 3, 4 ] # [ 1, 2, 3, 4 ] - note no duplicates

# Concatenation (order matters):
[ 1, 2, 3 ] + [ 2, 3, 4 ] # [ 1, 2, 3, 2, 3, 4 ]  # duplicates
[ 2, 3, 4 ] + [ 1, 2, 3 ] # [ 2, 3, 4, 1, 2, 3 ]

# Intersection:
[ 1, 2, 3 ] & [ 2, 3, 4 ] # [    2, 3    ]

# Symmetric diff (NAND)
[ 1, 2, 3 ] !& [ 2, 3, 4 ] # [ 1,       4 ]

# Difference (order matters)
[ 1, 2, 3 ] - [ 2, 3, 4 ] # [ 1          ]
[ 2, 3, 4 ] - [ 1, 2, 3 ] # [          4 ]

```

## File I/O

```weave
# reading data files is simple
fn display_csv(filename) {
  # builtin: open(filename, access_type)
  # csv_file pointer will be closed when it 
  #   - goes out of scope
  #   - is read to the end
  csv_file = open(filename, :read)

  # builtin: read(file, format)
  # Passing :csv as the format, we will get a parsed Container object.
  # supported formats: 
  #   - :json, :yaml, :toml, :ini
  # supported encodings: 
  #   - :utf8, :utf16
  # for everything else, there's
  #   - :binary
  csv = read(csv_file, :csv)

  print(csv[0].keys) # header turns into keys for CSVs
  first_col = csv[0].keys[0]

  # Print the value of each row's first column.
  csv.rows |> ^(x) { print(x[first_col]) }
}

# Writing files is simple too
fn write_to_csv(filename, data) {
  out_file = open(filename, :write)
  write(outfile, data, :csv)  # writes 'data' Container in CSV format
}

# Custom formats can be handled by passing parser function instead of a symbol
fn excel_to_container(raw_binary_data) { # raw_binary_data is a Container of Bytes. 
  # Left as an excersise for the reader
  excl_container # file contents returned as a Container object
}

fn read_an_excel_file(filename) {
  # Pass the parser function "excel_to_container" instead of a symbol
  # for custom parse handling
  excel_data = open(filename, excel_to_container)
end
```

## Shell Calls

```weave
# Execing a shell process and capturing its output is trivial

# Surround the entire call meant to be sent to the system with backticks
result = `ls *.csv`  # Returns a "Shell Result" Container. Contains console output and exit code

if result[:success] {
  print("csv files:")
  print(result[:output])
} else {
  print("failure:")
  print(result[:output])
}

# Bash Pipes work too
first_csvs = `ls *.csv | sort | head -n2`[:output] # grabs the first two CSV files in alphabetical order, then (assumes success) and captures output
```

