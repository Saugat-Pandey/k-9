# Why “K-9”?

We named this project **K-9** because it behaves a bit like a quick, loyal retrieval dog.  
You throw data at it, and it fetches it back **fast**, **reliably**, and without hesitation.  
Just like a trained K-9 unit, our key-value store stays focused, efficient, and always ready to retrieve.

# Description
At its core, K-9 is a **log-structured key-value store** written in Rust.  
On top of that storage engine, we are building a **terminal-based Notes application** using **ratatui**.

# What this project is (Lab9)

This repository contains two layers:

- `KvStore` (`src/lib.rs`)  
  A binary, log-structured key-value store with checksums, compaction, and zero-allocation iteration.

- `Notes` (`src/notes.rs` + `notes_tui`)  
  A real application that stores each note as a binary blob inside the KV store and exposes it via a TUI.

The goal is to demonstrate how a low level storage engine can power a real, user-facing application.

## Project status

This project is actively being developed.  
See [ToDo.md](ToDo.md) for the current roadmap and next steps.

# Notes anlegen

e.g.:

```bash
cargo run --bin notes_cli -- notes.db new "Titel 1" "Body 1"
cargo run --bin notes_cli -- notes.db new "Titel 2" "Body 2"
```

# Run the TUI

```bash
cargo run --bin notes_tui
```

Press `q` to quit.

# Run all tests

```bash
cargo test
```