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
    println!("  ? retract(person(alice)).");
    println!("  ? retract(edge(alice, knows, bob)).");
    println!("  ? retract(prop_name(alice, _)).");
}

fn eval_repl_source(db: &ZlfDatabase, source: &str) -> Result<serde_json::Value> {
    if source.starts_with('?') || source.contains(":-") {
        return Ok(serde_json::json!(db.query_prolog(source)?));
    }

    let facts = split_fact_sources(source);
    let count = facts.len();
    for fact_source in facts {
        let fact = PrologParser::parse_fact(&fact_source)?;
        db.apply_fact(&fact.head)?;
    }
    if count == 1 {
        Ok(serde_json::json!({ "applied": true }))
    } else {
        Ok(serde_json::json!({ "applied": count }))
    }
}

fn split_fact_sources(source: &str) -> Vec<String> {
    let mut facts = Vec::new();
    let mut start = 0usize;
    let mut in_string = false;
    for (idx, ch) in source.char_indices() {
        if ch == '"' {
            in_string = !in_string;
        }
        if ch == '.' && !in_string {
            let fact = source[start..=idx].trim();
            if !fact.is_empty() {
                facts.push(fact.to_string());
            }
            start = idx + 1;
        }
    }
    if facts.is_empty() && !source.trim().is_empty() {
        facts.push(source.trim().to_string());
    }
    facts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repl_accepts_multiple_facts_on_one_line() {
        let path = tempfile::tempdir().unwrap().keep();
        let db = ZlfDatabase::open(&path).unwrap();

        let output = eval_repl_source(&db, "node(a). node(b). follows(b, a).").unwrap();
        assert_eq!(output["applied"], 3);

        let rows = db.query_prolog("? follows(b, X).").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["X"], "a");
    }

    #[test]
    fn repl_fact_splitter_ignores_dots_inside_strings() {
        assert_eq!(
            split_fact_sources("node(a, [doc], { text: \"a.b\" }). node(b)."),
            vec![
                "node(a, [doc], { text: \"a.b\" }).".to_string(),
                "node(b).".to_string()
            ]
        );
    }
}
