use std::collections::HashSet;
use std::time::Instant;

use serde::Serialize;
use zlf_query::ZlfDatabase;

const RULES: &[(&str, &str)] = &[
    ("taxonomy_ancestor/2", "taxonomy_ancestor(X,Y) :- taxonomy_parent(X,Y)."),
    ("taxonomy_ancestor/2", "taxonomy_ancestor(X,Y) :- taxonomy_ancestor(X,Z), taxonomy_parent(Z,Y)."),
    ("taxonomy_descendant/2", "taxonomy_descendant(X,Y) :- taxonomy_parent(Y,X)."),
    ("taxonomy_descendant/2", "taxonomy_descendant(X,Y) :- taxonomy_descendant(X,Z), taxonomy_parent(Y,Z)."),
    ("taxonomy_distance_up/3", "taxonomy_distance_up(S,S,0)."),
    ("taxonomy_distance_up/3", "taxonomy_distance_up(S,Y,D) :- taxonomy_distance_up(S,X,D0), taxonomy_parent(X,Y), is(D,'+'(D0,1))."),
    ("taxonomy_lca/3", "taxonomy_lca(A,B,L) :- taxonomy_distance_up(A,L,DA), taxonomy_distance_up(B,L,DB), !."),
    ("taxonomic_distance/3", "taxonomic_distance(A,B,D) :- taxonomy_distance_up(A,L,DA), taxonomy_distance_up(B,L,DB), !, is(D,'+'(DA,DB))."),
];

#[derive(Serialize)]
struct Measurement {
    name: String,
    query: String,
    rows: usize,
    cold_ms: f64,
    warm_ms: Vec<f64>,
    checksum: u64,
}

#[derive(Serialize)]
struct DistanceResult {
    left: String,
    right: String,
    lca: Option<String>,
    distance: Option<i64>,
}

#[derive(Serialize)]
struct StressReport {
    measurements: Vec<Measurement>,
    taxonomy_distance: DistanceResult,
    table_metrics: zlf_prolog::wam::TableMetricsSnapshot,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("zlf-taxonomy-stress: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (db, left, right, iterations) = configured_database()?;
    let measurements = workloads(&left, &right)
        .into_iter()
        .map(|(name, query)| measure(&db, name, query, iterations))
        .collect::<Result<Vec<_>, _>>()?;
    let report = StressReport {
        measurements,
        taxonomy_distance: compute_distance(&db, &left, &right)?,
        table_metrics: db.table_metrics(),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn configured_database() -> Result<(ZlfDatabase, String, String, usize), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let [database, left, right, rest @ ..] = args.as_slice() else {
        return Err(
            "usage: zlf-taxonomy-stress <db> <left-tax-id> <right-tax-id> [iterations]".to_string(),
        );
    };
    let iterations = rest
        .first()
        .map_or(Ok(5), |value| value.parse::<usize>())
        .map_err(|error| error.to_string())?;
    let db = ZlfDatabase::open_existing(database).map_err(|error| error.to_string())?;
    install_rules(&db)?;
    for directive in [
        ":- table taxonomy_ancestor/2.",
        ":- table taxonomy_descendant/2.",
        ":- table taxonomy_distance_up/3.",
        ":- table taxonomy_lca/3.",
        ":- table taxonomic_distance/3.",
    ] {
        db.query_prolog(directive)
            .map_err(|error| error.to_string())?;
    }
    Ok((db, left.clone(), right.clone(), iterations))
}

fn workloads(left: &str, right: &str) -> [(&'static str, String); 8] {
    [
        ("name", format!("? prop_scientific_name(tax_{left}, Name).")),
        reverse_name_workload(),
        (
            "lineage",
            format!("? taxonomy_ancestor(tax_{left}, Ancestor)."),
        ),
        (
            "descendants",
            format!("? taxonomy_descendant(tax_{right}, Descendant)."),
        ),
        (
            "distance_up_left",
            format!("? taxonomy_distance_up(tax_{left}, Taxon, Distance)."),
        ),
        (
            "distance_up_right",
            format!("? taxonomy_distance_up(tax_{right}, Taxon, Distance)."),
        ),
        (
            "lca",
            format!("? taxonomy_lca(tax_{left}, tax_{right}, Lca)."),
        ),
        (
            "taxonomic_distance",
            format!("? taxonomic_distance(tax_{left}, tax_{right}, Distance)."),
        ),
    ]
}

fn reverse_name_workload() -> (&'static str, String) {
    (
        "name_reverse_homo_sapiens",
        "? prop_scientific_name(Taxon, \"Homo sapiens\").".to_string(),
    )
}

fn install_rules(db: &ZlfDatabase) -> Result<(), String> {
    let indicators = RULES
        .iter()
        .map(|(indicator, _)| *indicator)
        .collect::<HashSet<_>>();
    let mut existing = HashSet::new();
    for indicator in indicators {
        let query = format!("? current_predicate({indicator}).");
        if !db
            .query_prolog(&query)
            .map_err(|error| error.to_string())?
            .is_empty()
        {
            existing.insert(indicator);
        }
    }
    for (indicator, rule) in RULES {
        if !existing.contains(indicator) {
            db.query_prolog(rule).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn measure(
    db: &ZlfDatabase,
    name: &str,
    query: String,
    iterations: usize,
) -> Result<Measurement, String> {
    let started = Instant::now();
    let rows = db.query_prolog(&query).map_err(|error| error.to_string())?;
    let cold_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let expected_checksum = checksum(&rows);
    let mut warm_ms = Vec::new();
    for _ in 0..iterations {
        let started = Instant::now();
        let warm_rows = db.query_prolog(&query).map_err(|error| error.to_string())?;
        if checksum(&warm_rows) != expected_checksum {
            return Err(format!("non-deterministic answers for workload {name}"));
        }
        warm_ms.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    Ok(Measurement {
        name: name.to_string(),
        query,
        rows: rows.len(),
        cold_ms,
        warm_ms,
        checksum: expected_checksum,
    })
}

fn compute_distance(db: &ZlfDatabase, left: &str, right: &str) -> Result<DistanceResult, String> {
    let lca_rows = db
        .query_prolog(&format!("? taxonomy_lca(tax_{left}, tax_{right}, Lca)."))
        .map_err(|error| error.to_string())?;
    let distance_rows = db
        .query_prolog(&format!(
            "? taxonomic_distance(tax_{left}, tax_{right}, Distance)."
        ))
        .map_err(|error| error.to_string())?;
    Ok(DistanceResult {
        left: format!("tax_{left}"),
        right: format!("tax_{right}"),
        lca: lca_rows
            .first()
            .and_then(|row| row["Lca"].as_str())
            .map(str::to_string),
        distance: distance_rows
            .first()
            .and_then(|row| row["Distance"].as_i64()),
    })
}

fn checksum(rows: &[serde_json::Value]) -> u64 {
    serde_json::to_vec(rows)
        .unwrap_or_default()
        .into_iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
        })
}
