use quickmatch::Matcher;
use std::io::{self, Write};

fn main() {
    let products = vec![
        "Apple iPhone 15 Pro",
        "Apple MacBook Pro 16",
        "Apple AirPods Pro",
        "Samsung Galaxy S24",
        "Samsung Galaxy Tab",
        "Sony PlayStation 5",
        "Sony WH-1000XM5 Headphones",
        "Microsoft Surface Pro",
        "Microsoft Xbox Series X",
        "Dell XPS 13 Laptop",
        "Dell UltraSharp Monitor",
        "Logitech MX Master Mouse",
        "Logitech Mechanical Keyboard",
        "Canon EOS R5 Camera",
        "Nikon Z9 Camera",
        "GoPro Hero 12",
    ]
    .into_iter()
    .map(|s| s.to_lowercase())
    .collect::<Vec<_>>();

    let products_ref = products.iter().map(|s| s.as_str()).collect::<Vec<_>>();

    let matcher = Matcher::new(&products_ref);

    println!("Type to search (press Ctrl+C to exit):");
    println!("Try: 'apple', 'pro', 'laptop', 'headphones', etc.\n");

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let query = input.trim();

        if query.is_empty() {
            continue;
        }

        let results = matcher.matches(query, usize::MAX);

        if results.is_empty() {
            println!("  No matches found\n");
        } else {
            println!("  {} result(s):", results.len());
            for result in results {
                println!("    • {}", result);
            }
            println!();
        }
    }
}
