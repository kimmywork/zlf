use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;
use zlf_config::ZlfConfig;
use zlf_prolog::PrologParser;
use zlf_query::ZlfDatabase;

#[allow(clippy::too_many_lines)]
pub(crate) fn run_repl(path: Option<&str>) -> Result<()> {
    let config = ZlfConfig::load();
    let path = path.unwrap_or(&config.db_path);
    let db_path = Path::new(path);
    if !db_path.exists() {
        std::fs::create_dir_all(db_path)?;
    }
    let db = if db_path.join("storage").exists() {
        ZlfDatabase::open_existing(db_path)?
    } else {
        ZlfDatabase::open(db_path)?
    };

    println!("zlf Prolog REPL ({})", db_path.display());
    println!("Type ?goal., fact., or rule.  Commands: :help, :quit");
    let stdin = io::stdin();
    loop {
        print!("zlf> ");
        io::stdout().flush()?;
        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match line {
            ":quit" | ":exit" => break,
            ":help" => {
                println!("Examples:");
                println!("  ?person(X).");
                println!("  ?property(X, name, Value).");
                println!("  ?bm25(\"软件\", Node, Score).");
                println!("  node(alice, [person], {{ name: \"Alice\" }}).");
                println!("  knows(alice, bob).");
                println!("  friend(X, Y) :- knows(X, Y).");
            }
            source => match eval_repl_source(&db, source) {
                Ok(output) => println!("{}", serde_json::to_string_pretty(&output)?),
                Err(error) => eprintln!("error: {error}"),
            },
        }
    }
    Ok(())
}

fn eval_repl_source(db: &ZlfDatabase, source: &str) -> Result<serde_json::Value> {
    if source.starts_with('?') || source.contains(":-") {
        return Ok(serde_json::json!(db.query_prolog(source)?));
    }

    let fact = PrologParser::parse_fact(source)?;
    db.apply_fact(&fact.head)?;
    Ok(serde_json::json!({ "applied": true }))
}
