use std::path::Path;

use anyhow::Result;
use reedline::{DefaultPrompt, Reedline, Signal};
use zlf_config::ZlfConfig;
use zlf_prolog::PrologParser;
use zlf_query::ZlfDatabase;

pub(crate) fn run_repl(path: Option<&str>) -> Result<()> {
    let db = open_repl_database(path)?;
    let mut editor = Reedline::create();
    let prompt = DefaultPrompt::default();

    println!("Type ?goal., fact., or rule.  Commands: :help, :quit");
    loop {
        match editor.read_line(&prompt)? {
            Signal::Success(line) if !handle_repl_line(&db, line.trim())? => break,
            Signal::Success(_) | Signal::CtrlC => continue,
            Signal::CtrlD => break,
        }
    }
    Ok(())
}

fn open_repl_database(path: Option<&str>) -> Result<ZlfDatabase> {
    let config = ZlfConfig::load();
    let path = path.unwrap_or(&config.db_path);
    let db_path = Path::new(path);
    if !db_path.exists() {
        std::fs::create_dir_all(db_path)?;
    }
    println!("zlf Prolog REPL ({})", db_path.display());
    if db_path.join("storage").exists() {
        Ok(ZlfDatabase::open_existing(db_path)?)
    } else {
        Ok(ZlfDatabase::open(db_path)?)
    }
}

fn handle_repl_line(db: &ZlfDatabase, line: &str) -> Result<bool> {
    if line.is_empty() {
        return Ok(true);
    }
    match line {
        ":quit" | ":exit" => return Ok(false),
        ":help" => print_help(),
        source => match eval_repl_source(db, source) {
            Ok(output) => println!("{}", serde_json::to_string_pretty(&output)?),
            Err(error) => eprintln!("error: {error}"),
        },
    }
    Ok(true)
}

fn print_help() {
    println!("Examples:");
    println!("  ?person(X).");
    println!("  ?property(X, name, Value).");
    println!("  ?bm25(\"软件\", Node, Score).");
    println!("  node(alice, [person], {{ name: \"Alice\" }}).");
    println!("  knows(alice, bob).");
    println!("  friend(X, Y) :- knows(X, Y).");
}

fn eval_repl_source(db: &ZlfDatabase, source: &str) -> Result<serde_json::Value> {
    if source.starts_with('?') || source.contains(":-") {
        return Ok(serde_json::json!(db.query_prolog(source)?));
    }

    let fact = PrologParser::parse_fact(source)?;
    db.apply_fact(&fact.head)?;
    Ok(serde_json::json!({ "applied": true }))
}
