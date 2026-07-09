use std::fs;
use std::path::{Path, PathBuf};

use zlf_index::{BM25Index, VectorIndex};
use zlf_prolog::wam::{
    BlockingEmbeddingProvider, CompositeFactProvider, Embedder, EmbeddingWorker, IndexFactProvider,
    IndexedStorageFactWriter, PersistentEmbeddingQueue, StorageFactProvider, StorageRuleStore,
    WamRuntime,
};
use zlf_prolog::{PrologParser, Term};
use zlf_storage::Storage;

struct FakeWikiEmbedder;

impl Embedder for FakeWikiEmbedder {
    fn model(&self) -> &str {
        "fake-wiki"
    }

    fn embed(&self, text: &str) -> zlf_prolog::wam::WamResult<Vec<f32>> {
        Ok(vec![
            text.len() as f32,
            text.bytes().next().unwrap_or(0) as f32,
            1.0,
        ])
    }
}

#[test]
#[ignore = "requires local wiki markdown folder"]
fn wiki_markdown_full_pipeline_with_compiled_rules_and_worker() {
    let wiki_dir = wiki_dir();
    let files = markdown_files(&wiki_dir);
    assert!(!files.is_empty(), "no markdown files under {wiki_dir:?}");

    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(dir.path().join("db")).unwrap();
    let bm25 = BM25Index::open(dir.path().join("bm25")).unwrap();
    let vector = VectorIndex::open(dir.path().join("vector")).unwrap();
    let writer = IndexedStorageFactWriter::new(&storage).with_bm25(&bm25);
    let queue = PersistentEmbeddingQueue::new(&storage);
    let embedder = wiki_embedder();

    let mut first_id = String::new();
    let mut first_query = String::new();
    for (index, file) in files.iter().enumerate() {
        let content = fs::read_to_string(file).unwrap();
        let title = file.file_stem().unwrap().to_string_lossy().to_string();
        let id = format!("wiki_doc_{index}");
        if index == 0 {
            first_id = id.clone();
            first_query = bm25_query_token(&title);
        }
        writer
            .apply_fact(&wiki_node(&id, &title, file, &content))
            .unwrap();
        queue.enqueue(&id, &content).unwrap();
    }

    let worker = EmbeddingWorker::new(&queue, embedder.as_ref(), &vector)
        .with_poll_interval(std::time::Duration::from_millis(1));
    assert_eq!(worker.run_until_idle(1).unwrap(), files.len());
    assert!(queue.pending().unwrap().is_empty());

    let store = StorageRuleStore::new(&storage);
    store
        .add_rule(&rule(
            "wiki_doc(Doc, Title) :- document(Doc), prop_title(Doc, Title).",
        ))
        .unwrap();

    let storage_provider = StorageFactProvider::new(&storage);
    let index_provider = IndexFactProvider::new()
        .with_bm25(&bm25)
        .with_vector(&vector);
    let provider = CompositeFactProvider::new()
        .with(&storage_provider)
        .with(&index_provider);
    let mut runtime = WamRuntime::new(16);
    for artifact in store.all_rules().unwrap() {
        runtime.add_compiled_rule(artifact);
    }

    let docs = runtime
        .query_all_with_provider(&term("wiki_doc(Doc, Title)"), &provider)
        .unwrap();
    assert_eq!(docs.len(), files.len());

    let bm25_rows = runtime
        .query_all_with_provider(&bm25_query(&first_query), &provider)
        .unwrap();
    assert!(!bm25_rows.is_empty());

    let vector_rows = runtime
        .query_all_with_provider(
            &term(&format!("vector_similar({first_id}, Node, Score)")),
            &provider,
        )
        .unwrap();
    assert!(vector_rows
        .iter()
        .any(|row| row.get("Node") == Some(&atom(&first_id))));
}

fn wiki_embedder() -> Box<dyn Embedder> {
    match std::env::var("ZLF_WIKI_EMBEDDER").as_deref() {
        Ok("ollama") => Box::new(
            BlockingEmbeddingProvider::ollama_bge_m3(
                std::env::var("OLLAMA_ENDPOINT")
                    .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            )
            .unwrap(),
        ),
        _ => Box::new(FakeWikiEmbedder),
    }
}

fn wiki_dir() -> PathBuf {
    std::env::var("ZLF_WIKI_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var_os("HOME").unwrap()).join("workspace/docs/wiki/content")
        })
}

fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_markdown(dir, &mut files);
    files.sort();
    files
}

fn collect_markdown(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_markdown(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            files.push(path);
        }
    }
}

fn wiki_node(id: &str, title: &str, path: &Path, content: &str) -> Term {
    Term::Compound {
        name: "node".to_string(),
        args: vec![
            atom(id),
            Term::List(vec![atom("document"), atom("markdown")]),
            Term::Object(vec![
                ("title".to_string(), Term::String(title.to_string())),
                ("path".to_string(), Term::String(path.display().to_string())),
                ("content".to_string(), Term::String(content.to_string())),
            ]),
        ],
    }
}

fn bm25_query_token(title: &str) -> String {
    title
        .split(|ch: char| !ch.is_alphanumeric())
        .find(|token| !token.is_empty())
        .unwrap_or(title)
        .to_lowercase()
}

fn bm25_query(query: &str) -> Term {
    Term::Compound {
        name: "bm25".to_string(),
        args: vec![
            Term::String(query.to_string()),
            Term::Variable("Node".to_string()),
            Term::Variable("Score".to_string()),
        ],
    }
}

fn term(source: &str) -> Term {
    PrologParser::parse_term(source).unwrap()
}

fn rule(source: &str) -> zlf_prolog::PrologRule {
    PrologParser::parse_rule(source).unwrap()
}

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
}
