# Brilho 🌟
Brilho is a fast application that converts markdown files to Anki cards.

# Setup

## With Nix
```bash
nix run github:arcticlimer/brilho --target <directory>
```

## With cargo
```bash
cargo build --release
./target/release/brilho --target <directory>
```

# Usage
Currently brilho expects your markdown to be in a format similar to [that](./others/test.md)

Even though you don't follow this specific format, brilho tries its best to make
markdown written in a similar way to look nice as Anki cards.

## Showcase
<img width="750px" alt="Shows brilho's usage overview" src="./others/brilho.jpg">

After the `csv` file is generated, you can just go into Anki and import it in
the **Import** tab.
> Remember to enable HTML when importing

# Benchmarks

Some simple benchmarks were made using [this repository]( https://github.com/insaneyilin/leetcode_anki).

## Brilho (4.1 milliseconds)
```sh
[nix-shell:~/projects/brilho]$ hyperfine "./target/release/brilho --target ../leetcode_anki/"
Benchmark #1: ./target/release/brilho --target ../leetcode_anki/
  Time (mean ± σ):       4.1 ms ±   0.6 ms    [User: 3.0 ms, System: 4.8 ms]
  Range (min … max):     2.8 ms …   5.8 ms    390 runs
```
## [Mdanki](https://github.com/ashlinchak/mdanki) (4.5 seconds)
```sh
[nix-shell:/tmp/stub/node_modules/mdanki]$ hyperfine "./src/index.js ~/projects/leetcode_anki/ ./result.apkg"
Benchmark #1: ./src/index.js ~/projects/leetcode_anki/ ./result.apkg
  Time (mean ± σ):      4.540 s ±  0.271 s    [User: 2.614 s, System: 0.097 s]
  Range (min … max):    4.318 s …  5.056 s    10 runs
```
> Note: mdanki was running with the [memory limit workaround](https://github.com/ashlinchak/mdanki#memory-limit).

# Philosophy
- It should stick to supporting mainly common markdown files
- It should be fast and give you a chance to review its output
- It should be minimal and only convert markdown files into anki cards

# Planned features
- Images support
- LaTeX support
- URLs support
- Backlink support
- Use nested headings context in generated cards
- Tests
- Logging

# Contributing
Feel free to open issues and pull requests!

If you want to help with development, you can access the Nix development
environment by running the `nix develop` command.
