use std::path::{Path, PathBuf};

use zlf_prolog::bulk_pack::{compile_fact_files, load_fact_pack, BulkCompileOptions};
use zlf_storage::Storage;

fn main() {
    if let Err(error) = run(std::env::args().skip(1).collect()) {
        eprintln!("zlf-bulk: {error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
        [command, output, inputs @ ..] if command == "compile" && !inputs.is_empty() => {
            compile(output, inputs)
        }
        [command, database, pack] if command == "load" => load(database, pack),
        _ => Err(
            "usage: zlf-bulk compile <output.zlfpack> <facts.pl>... | zlf-bulk load <db> <pack>"
                .to_string(),
        ),
    }
}

fn compile(output: &str, inputs: &[String]) -> Result<(), String> {
    let inputs = inputs.iter().map(PathBuf::from).collect::<Vec<_>>();
    let manifest = compile_fact_files(&inputs, Path::new(output), &BulkCompileOptions::default())
        .map_err(|error| error.to_string())?;
    println!(
        "compiled {} facts into {} records",
        manifest.fact_counts.values().sum::<u64>(),
        manifest.record_count
    );
    Ok(())
}

fn load(database: &str, pack: &str) -> Result<(), String> {
    let database = Path::new(database);
    std::fs::create_dir_all(database).map_err(|error| error.to_string())?;
    let storage_path = database.join("storage");
    let storage = if storage_path.exists() {
        Storage::open_existing(&storage_path)
    } else {
        Storage::open(&storage_path)
    }
    .map_err(|error| error.to_string())?;
    let report =
        load_fact_pack(&storage, Path::new(pack), 50_000).map_err(|error| error.to_string())?;
    println!(
        "loaded {} records in {} batches (already_loaded={})",
        report.records_written, report.batches_written, report.already_loaded
    );
    Ok(())
}
