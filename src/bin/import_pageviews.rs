use kv_store::{KvStore, Key, OwnedValue};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut store = KvStore::new();

    let pageview_files = vec![
        "data/pageviews-1.txt",
        "data/pageviews-2.txt",
    ];

    let start = Instant::now();

    for (i, path) in pageview_files.iter().enumerate() {
        println!("Processing file: {path}");

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();

        let first_file = i == 0;

        loop {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                break; 
            }

            let mut tokens = line.split_ascii_whitespace();
            let domain_code = match tokens.next() {
                Some(s) => s,
                None => continue,
            };
            let page_name = match tokens.next() {
                Some(s) => s,
                None => continue,
            };
            let view_count_str = match tokens.next() {
                Some(s) => s,
                None => continue,
            };

            let view_count: i64 = match view_count_str.parse() {
                Ok(n) => n,
                Err(_) => continue,
            };

            let key_string = format!("{}/{}", domain_code, page_name);
            let key = Key::Text(key_string.clone());

            if first_file {
                store.insert(key, OwnedValue::Integer(view_count));
            } else {
                if let Some(prev) = store.get_owned(&key)? {
                    if let OwnedValue::Integer(prev_count) = prev {
                        store.insert(key, OwnedValue::Integer(prev_count + view_count));
                    }
                } else {
                    store.insert(key, OwnedValue::Integer(view_count));
                }
            }
        }
    }

    let duration = start.elapsed();
    println!("Finished importing in {:?}", duration);

    let sample_keys = vec![
    "en/Main_Page",
    "de/Wikipedia:Hauptseite",
    "commons/File:Example.jpg",
    ];

    for k in sample_keys {
        let key = Key::Text(k.to_string());
        println!("{} -> {:?}", k, store.get_owned(&key)?);
    }

    Ok(())
}
