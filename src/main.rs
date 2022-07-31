extern crate comrak;
use async_std::fs;
use futures::future::join_all;
use regex::Regex;
use std::cell::RefCell;
use std::env;
use std::fs::File;
use std::io::Write;

use comrak::arena_tree::Node;
use comrak::nodes::{Ast, NodeValue};
use comrak::{parse_document, Arena, ComrakOptions};
use walkdir::WalkDir;

#[derive(Debug)]
struct Header {
    pub level: u32,
    pub title: String,
    pub content: String,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            level: 0,
            title: "".to_string(),
            content: "".to_string(),
        }
    }
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
            front: String::new(),
            back: String::new(),
            bullets: Vec::with_capacity(10),
            links: Vec::with_capacity(5),
        }
    }
}

#[derive(Debug)]
struct Parser {
    current_header: Header,
    current_card: Card,
    cards: Vec<Card>,
    in_header: bool,
    in_list: bool,
    expecting_title: bool,
}

impl Parser {
    pub fn parse_from_root<'a>(&mut self, node: &'a Node<'a, RefCell<Ast>>) -> Vec<Card> {
        self.parse_node_recursive(node);
        self.finish();
        self.cards.clone()
    }

    fn parse_node_recursive<'a>(&mut self, node: &'a Node<'a, RefCell<Ast>>) {
        match &mut node.data.borrow_mut().value {
            // TODO: Build heading levels context in order to use subtopics in it like "Topic - <question>"
            &mut NodeValue::Heading(ref mut heading) => self.heading(heading.level),
            &mut NodeValue::List(ref mut _list) => self.list(),
            &mut NodeValue::Text(ref mut text) => {
                let content = String::from_utf8_lossy(text);
                self.text(&content)
            }
            _ => (),
        }

        // TODO: Study about how this tree representation works, currently
        // it seems very sketchy
        for c in node.children() {
            self.parse_node_recursive(c)
        }
    }

    fn finish(&mut self) {
        if self.current_header.level == 0
            || self.current_header.content.is_empty()
            || self.current_header.title.is_empty()
        {
            return;
        }

        self.cards.push(self.current_card.clone());
    }

    fn heading(&mut self, level: u32) {
        if self.in_header {
            let card = Card {
                front: self.current_header.title.to_string(),
                back: self.current_header.content.to_string(),
                ..Default::default()
            };
            self.cards.push(card);
        }

        self.expecting_title = true;
        self.in_header = true;
        self.current_header.level = level;
    }

    fn list(&mut self) {
        if !self.in_header {
            return;
        }

        self.in_list = true;
    }

    fn text(&mut self, content: &str) {
        // TODO: Support backlinks
        lazy_static::lazy_static! {
            static ref IMAGE_BACKLINK: Regex = Regex::new(r"\[\[.*.png]]").unwrap();
            static ref LINK: Regex = Regex::new(r"\[(.*)]\((.*)\)").unwrap();
        }

        // We parse links manually, since currently the comrak link parser seems
        // very unreliable, e.g [links like](this) are losing their title metadata
        dbg!(content, LINK.captures(&content));
        if let Some(captures) = LINK.captures(content) {
            let title = captures.get(0).unwrap();
            let url = captures.get(1).unwrap();
            dbg!(title, url);
            // for capture in captures.iter() {
            //     if let Some(mat) = capture {
            //         self.current_card.links.push(mat.as_str().to_string());
            //     }
            // }
        }

        // if let Some(captures) = IMAGE_BACKLINK.captures(content) {
        //     // TODO: Maybe use helper function for this
        //     for capture in captures.iter() {
        //         if let Some(mat) = capture {
        //             self.current_card.images.push(mat.as_str().to_string());
        //         }
        //     }
        // }

        if self.expecting_title {
            self.current_header.title = content.to_string();
            self.current_header.content = String::new();
            self.expecting_title = false;
            return;
        }

        // TODO: we cannot assume every text token is separated by newlines
        // that means not appending automatically a </br>
        if self.in_list {
            self.current_card.bullets.push(content.to_string());
            self.in_list = false;
            return;
        }
        self.current_header.content.push_str(&content);
        self.current_header.content.push_str("</br>")
    }
}

impl Default for Parser {
    fn default() -> Self {
        Parser {
            current_card: Card::default(),
            current_header: Header::default(),
            cards: Vec::with_capacity(50),
            in_header: false,
            in_list: false,
            expecting_title: false,
        }
    }
}

#[derive(Debug)]
struct CsvBuilder<'a> {
    buffer: &'a mut String,
}

impl<'a> CsvBuilder<'a> {
    fn new(buffer: &'a mut String) -> Self {
        Self { buffer }
    }

    pub fn add_card(self: &mut Self, card: &Card) {
        self.buffer.push_str(&card.front);
        self.buffer.push(',');
        self.buffer.push_str(&card.back);
        self.buffer.push('\n');
    }

    pub fn collect(self: &mut Self) -> &String {
        self.buffer
    }
}

async fn create_computation(filepath: String) -> Vec<Card> {
    let arena = Arena::new();
    let content = fs::read_to_string(&filepath).await.unwrap();
    let root = parse_document(&arena, &content, &ComrakOptions::default());

    let mut parser = Parser::default();
    parser.parse_from_root(root)
}

#[async_std::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("usage: <program> <vault_directory>")
    }
    let directory = &args[1];

    let entries = WalkDir::new(directory)
        .into_iter()
        .filter_map(|e| e.ok()) // Ignores errors (such as permission errors)
        .filter(|e| e.file_name().to_str().unwrap().ends_with(".md")); // markdown only files

    let mut futures = Vec::with_capacity(500);
    for entry in entries {
        let path = entry.path().to_str().unwrap();
        futures.push(create_computation(path.to_owned()));
    }

    let cards = join_all(futures).await;

    let buffer = &mut String::new();
    let mut csv_builder = CsvBuilder::new(buffer);
    for card in cards.into_iter().flatten() {
        // dbg!(&card);
        csv_builder.add_card(&card);
    }

    let result = csv_builder.collect();
    let mut file = File::create("anki_cards.csv").unwrap();
    file.write_all(result.as_bytes()).unwrap();
}
