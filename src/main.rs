use async_std::fs;
use clap::Parser;
use futures::future::join_all;
use pulldown_cmark::{HeadingLevel, Options};
use std::error::Error;
use std::fs::File;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;
use walkdir::WalkDir;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    // TODO: Use lazy_static
    // Load these once at the start of your program
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

#[derive(Debug, Clone)]
struct Card {
    pub front: String,
    pub back: String,
    pub bullets: Vec<String>,
    pub links: Vec<String>,
}

impl Default for Card {
    fn default() -> Self {
        Self {
            front: String::with_capacity(100),
            back: String::with_capacity(300),
            bullets: Vec::with_capacity(10),
            links: Vec::with_capacity(5),
        }
    }
}

#[derive(Debug)]
struct CodeBlock {
    language: Option<String>,
    code: String,
}

impl Default for CodeBlock {
    fn default() -> Self {
        Self {
            language: None,
            code: String::with_capacity(1000),
        }
    }
}

#[derive(Debug)]
struct MarkdownParser {
    current_card: Card,
    current_list_item: String,
    current_code_block: CodeBlock,
    cards: Vec<Card>,
    parsing: bool,
    in_list: bool,
    in_list_item: bool,
    expecting_title: bool,
    in_code_block: bool,
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

impl MarkdownParser {
    pub fn parse(&mut self, input: &str) -> Vec<Card> {
        let parser = pulldown_cmark::Parser::new_ext(input, Options::empty());

        for event in parser {
            match event {
                pulldown_cmark::Event::SoftBreak | pulldown_cmark::Event::HardBreak => {
                    if self.parsing {
                        self.current_card.back.push_str("<br>")
                    }
                }
                pulldown_cmark::Event::Text(value) => self.text(&value),
                pulldown_cmark::Event::Code(code) => {
                    if code.is_empty() {
                        continue;
                    }

                    let result = format!("<bold>{}</bold>", code);
                    if self.in_list_item {
                        self.current_list_item.push_str(&result);
                    } else {
                        self.current_card.back.push_str(&result)
                    }
                }
                pulldown_cmark::Event::Start(tag) => match tag {
                    pulldown_cmark::Tag::Heading(level, _content, _classes) => {
                        self.expecting_title = true;
                        self.heading(heading_level_to_u8(level))
                    }
                    pulldown_cmark::Tag::List(_idx) => self.list(),
                    pulldown_cmark::Tag::BlockQuote | pulldown_cmark::Tag::Item => {
                        self.in_list_item = true;
                    }
                    pulldown_cmark::Tag::CodeBlock(kind) => {
                        match kind {
                            pulldown_cmark::CodeBlockKind::Indented => {}
                            pulldown_cmark::CodeBlockKind::Fenced(language) => {
                                if language.is_empty() {
                                    continue;
                                }

                                self.current_code_block.language = Some(language.to_string())
                            }
                        }

                        self.in_code_block = true; // TODO: This flag might not be needed
                    }
                    _ => (),
                },
                pulldown_cmark::Event::End(tag) => match tag {
                    pulldown_cmark::Tag::Link(_link_type, url, title) => {
                        self.link(&title, &url);
                    }
                    pulldown_cmark::Tag::List(_idx) => {
                        self.in_list = false;
                    }
                    pulldown_cmark::Tag::BlockQuote | pulldown_cmark::Tag::Item => {
                        self.current_card
                            .bullets
                            .push(self.current_list_item.clone());
                        self.current_list_item = String::with_capacity(100);
                        self.in_list_item = false;
                    }
                    pulldown_cmark::Tag::Heading(_, _, _) => {
                        self.expecting_title = false;
                    }
                    pulldown_cmark::Tag::CodeBlock(_) => {
                        let result = {
                            let lang = match &self.current_code_block.language {
                                Some(lang) => lang,
                                None => "",
                            };
                            let syntax = SYNTAX_SET
                                .find_syntax_by_extension(&lang)
                                .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

                            let theme = &THEME_SET.themes["base16-ocean.dark"]; // TODO: Make theme configurable
                                                                                // TODO: remove theme background
                            highlighted_html_for_string(
                                &self.current_code_block.code,
                                &SYNTAX_SET,
                                syntax,
                                theme,
                            )
                            .unwrap()
                        };

                        let left_aligned = format!(
                            "
                        <div style=\"text-align: left\">
                            {}
                        </div>
                        ",
                            result
                        );

                        if self.in_list_item {
                            self.current_list_item.push_str(&left_aligned)
                        } else {
                            self.current_card.back.push_str(&left_aligned)
                        }

                        self.current_code_block = CodeBlock::default();
                        self.in_code_block = false
                    }
                    _ => (),
                },
                _ => (),
            }
        }

        self.add_current_card();
        self.cards.clone()
    }

    fn link(&mut self, _title: &str, url: &str) {
        self.current_card.links.push(url.to_string());
    }

    fn add_current_card(&mut self) {
        if self.current_card_valid() {
            // Preprocessing
            for link in &self.current_card.bullets {
                if link.is_empty() {
                    continue;
                }

                self.current_card.back.push_str("<br>- ");
                self.current_card.back.push_str(link)
            }

            self.cards.push(self.current_card.clone()); // TODO: try to avoid cloning here
            self.current_card = Card::default()
        }
    }

    fn current_card_valid(&mut self) -> bool {
        if !self.current_card.front.is_empty() && !self.current_card.back.is_empty() {
            return true;
        }

        if !self.current_card.front.is_empty() || !self.current_card.bullets.is_empty() {
            return true;
        }

        false
    }

    fn heading(&mut self, _level: u8) {
        if self.parsing {
            self.add_current_card()
        }

        self.parsing = true;
    }

    fn list(&mut self) {
        if !self.parsing {
            return;
        }

        self.in_list = true;
    }

    fn text(&mut self, content: &str) {
        if self.expecting_title {
            self.current_card.front = content.to_string();
            return;
        }

        // TODO: Support backlinks
        if self.in_code_block {
            self.current_code_block.code.push_str(content);
            return;
        }

        if self.in_list_item {
            self.current_list_item.push_str(content);
            return;
        }

        self.current_card.back.push_str(content);
    }
}

impl Default for MarkdownParser {
    fn default() -> Self {
        MarkdownParser {
            current_card: Card::default(),
            current_list_item: String::with_capacity(100),
            current_code_block: CodeBlock::default(),
            cards: Vec::with_capacity(50),
            parsing: false,
            in_list: false,
            in_list_item: false,
            expecting_title: false,
            in_code_block: false,
        }
    }
}

async fn create_computation(filepath: String) -> Vec<Card> {
    if let Ok(content) = fs::read_to_string(&filepath).await {
        let mut parser = MarkdownParser::default();
        return parser.parse(&content);
    }

    Vec::new()
}

fn is_markdown(extension: &str) -> bool {
    extension.ends_with(".md") || extension.ends_with(".markdown")
}

/// Simple program to greet a person
#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Directory or file containing markdown content
    #[clap(short, long, value_parser)]
    target: String,

    /// Name of the outputted CSV file
    #[clap(short, long, value_parser, default_value = "anki_cards.csv")]
    output_file: String,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let entries = WalkDir::new(args.target)
        .into_iter()
        .filter_map(|e| e.ok()) // Ignores errors (such as permission errors)
        .filter(|e| is_markdown(e.file_name().to_str().unwrap())); // markdown only files

    let mut futures = Vec::with_capacity(500);
    for entry in entries {
        let path = entry.path().to_str().unwrap();
        futures.push(create_computation(path.to_owned()));
    }

    let cards = join_all(futures).await.concat();
    let file = File::create(&args.output_file).unwrap();

    let mut wtr = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .quote_style(csv::QuoteStyle::NonNumeric)
        .from_writer(file);

    for card in &cards {
        wtr.write_record(&[&card.front, &card.back])?;
    }

    println!(
        "Succesfully created {} cards into {}",
        cards.len(),
        args.output_file
    );

    Ok(())
}
