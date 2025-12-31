use kv_store::notes::NoteStore;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        print_usage();
        process::exit(1);
    }
    
    let file = &args[1];
    let command = &args[2];
    
    let result = match command.as_str() {
        "list" => cmd_list(file),
        "new" => {
            if args.len() < 5 {
                eprintln!("Error: 'new' requires <title> and <body>");
                print_usage();
                process::exit(1);
            }
            cmd_new(file, &args[3], &args[4])
        }
        "show" => {
            if args.len() < 4 {
                eprintln!("Error: 'show' requires <id>");
                print_usage();
                process::exit(1);
            }
            cmd_show(file, &args[3])
        }
        _ => {
            eprintln!("Error: unknown command '{}'", command);
            print_usage();
            process::exit(1);
        }
    };
    
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn print_usage() {
    eprintln!("Usage: notes_cli <FILE> <COMMAND> [ARGS...]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  list                  List all notes");
    eprintln!("  new <title> <body>    Create a new note");
    eprintln!("  show <id>             Show a note by ID");
}

fn cmd_list(file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let store = NoteStore::open(file)?;
    let metas = store.list_meta()?;
    
    for meta in metas {
        println!("{}  {}", meta.id, meta.title);
    }
    
    Ok(())
}

fn cmd_new(file: &str, title: &str, body: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut store = NoteStore::open(file)?;
    let id = store.create(title.to_string(), body.to_string())?;
    store.save(file)?;
    
    println!("created {}", id);
    
    Ok(())
}

fn cmd_show(file: &str, id_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    let id: u64 = id_str.parse()
        .map_err(|_| format!("invalid id: {}", id_str))?;
    
    let store = NoteStore::open(file)?;
    
    match store.get(id)? {
        Some(note) => {
            println!("{}", note.title);
            println!("{}", note.body);
        }
        None => {
            println!("not found");
        }
    }
    
    Ok(())
}
