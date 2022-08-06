# This is a test card
As you can see,

- It can support lists
- It can support lists
> And also notes

# This is another card
```py
# it supports code blocks
print("hello world")
```

<!-- ## This is a simple card -->
<!-- Of course it can be just simple text. -->

## Rust highlight
```rs
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

// Load these once at the start of your program
let ps = SyntaxSet::load_defaults_newlines();
let ts = ThemeSet::load_defaults();

let syntax = ps.find_syntax_by_extension("rs").unwrap();
let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
let s = "pub struct Wow { hi: u64 }\nfn blah() -> u64 {}";
for line in LinesWithEndings::from(s) {
    let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
    let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
    print!("{}", escaped);
}
```

## Foobar

- `test code`
-   ```py
    class Foobar:
        def __init__(self):
            return self

        def add(self, n):
            return n + 1
    ```
- yeah man
