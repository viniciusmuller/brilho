extern crate comrak;
use async_std::fs;
use futures::future::join_all;
use std::cell::RefCell;
use std::{env, vec};

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

#[derive(Debug, Clone)]
struct Card {
    pub front: String,
    pub back: String,
}

#[derive(Debug)]
struct Parser {
    current_header: Header,
    cards: Vec<Card>,
    in_header: bool,
    expecting_title: bool,
}

impl Parser {
    pub fn parse_from_root<'a>(self: &mut Self, node: &'a Node<'a, RefCell<Ast>>) -> Vec<Card> {
        self.parse_node_recursive(node);
        self.finish();
        self.cards.clone()
    }

    fn parse_node_recursive<'a>(self: &mut Self, node: &'a Node<'a, RefCell<Ast>>) {
        match &mut node.data.borrow_mut().value {
            // TODO: Parse links
            &mut NodeValue::Heading(ref mut heading) => self.heading(heading.level),
            &mut NodeValue::Text(ref mut text) => {
                let content = String::from_utf8_lossy(text);
                self.text(&content)
            }
            _ => (),
        }

        for c in node.children() {
            self.parse_node_recursive(c)
        }
    }

    fn finish(self: &mut Self) {
        if self.current_header.level == 0
            || self.current_header.content.is_empty()
            || self.current_header.title.is_empty()
        {
            return;
        }

        let card = self.build_card();
        self.cards.push(card);
    }

    fn heading(&mut self, level: u32) {
        if self.in_header {
            let card = self.build_card();
            self.cards.push(card);
        }

        self.expecting_title = true;
        self.in_header = true;
        self.current_header.level = level;
    }

    fn text(&mut self, content: &str) {
        if self.expecting_title {
            self.current_header.title = content.to_string();
            self.current_header.content = String::new();
            self.expecting_title = false;
            return;
        }

        self.current_header
            .content
            .push_str(&format!("{}\n", content));
    }

    fn build_card(&mut self) -> Card {
        Card {
            front: self.current_header.title.to_string(),
            back: self.current_header.content.to_string(),
        }
    }
}

impl Default for Parser {
    fn default() -> Parser {
        Parser {
            current_header: Header {
                level: 0,
                title: "".to_string(),
                content: "".to_string(),
            },
            cards: vec![],
            in_header: false,
            expecting_title: false,
        }
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

    let mut futures = vec![];
    for entry in entries {
        let path = entry.path().to_str().unwrap();
        futures.push(create_computation(path.to_owned()));
    }

    let cards = join_all(futures).await;
    for card in cards.into_iter().flatten() {
        dbg!(card);
    }
}
