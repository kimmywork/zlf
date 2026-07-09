use std::path::{Path, PathBuf};

use anyhow::Result;
use zlf_core::{Edge, Node};
use zlf_query::ZlfDatabase;

use crate::values::json_to_properties;

#[allow(clippy::too_many_lines)]
pub(crate) fn import_json(db: &ZlfDatabase, file: &str) -> Result<usize, String> {
    let path = Path::new(file);
    if path.is_dir() {
        return import_markdown_folder(db, path);
    }
    if is_markdown_path(path) {
        return import_markdown_file(db, path);
    }

    let content =
        std::fs::read_to_string(file).map_err(|e| format!("Failed to read file: {}", e))?;

    let data: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON: {}", e))?;

    let mut count = 0;

    if let Some(nodes) = data.get("nodes").and_then(|n| n.as_array()) {
        for node_data in nodes {
            let labels = node_data
                .get("labels")
                .and_then(|l| l.as_array())
                .map(|l| {
                    l.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let properties = node_data
                .get("properties")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            let id = node_data.get("id").and_then(|v| v.as_str());

            let node = if let Some(id) = id {
                Node::with_id(id.to_string(), labels, json_to_properties(properties))
            } else {
                Node::new(labels, json_to_properties(properties))
            };
            match db.add_node(node) {
                Ok(_) => count += 1,
                Err(e) => eprintln!("Warning: Failed to import node: {}", e),
            }
        }
    }

    if let Some(edges) = data.get("edges").and_then(|e| e.as_array()) {
        for edge_data in edges {
            let edge_type = edge_data
                .get("edge_type")
                .or_else(|| edge_data.get("type"))
                .and_then(|e| e.as_str())
                .unwrap_or("")
                .to_string();

            let source = edge_data
                .get("source")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();

            let target = edge_data
                .get("target")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            let properties = edge_data
                .get("properties")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            let id = edge_data.get("id").and_then(|v| v.as_str());

            let edge = if let Some(id) = id {
                Edge::with_id(
                    id.to_string(),
                    edge_type,
                    source,
                    target,
                    json_to_properties(properties),
                )
            } else {
                Edge::new(edge_type, source, target, json_to_properties(properties))
            };
            match db.add_edge(edge) {
                Ok(_) => count += 1,
                Err(e) => eprintln!("Warning: Failed to import edge: {}", e),
            }
        }
    }

    Ok(count)
}

fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false)
}

fn import_markdown_folder(db: &ZlfDatabase, folder: &Path) -> Result<usize, String> {
    let mut files = Vec::new();
    collect_markdown_files(folder, &mut files)?;
    let mut count = 0;
    for file in files {
        count += import_markdown_file(db, &file)?;
    }
    Ok(count)
}

fn collect_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, files)?;
        } else if is_markdown_path(&path) {
            files.push(path);
        }
    }
    files.sort();
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn import_markdown_file(db: &ZlfDatabase, file: &Path) -> Result<usize, String> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("Failed to read markdown file {}: {}", file.display(), e))?;
    let title = content
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .unwrap_or_else(|| {
            file.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("untitled")
        });
    let canonical = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let doc_id = format!(
        "doc:{}",
        canonical
            .to_string_lossy()
            .replace(['/', '\\', ' ', ':'], "_")
    );

    let mut count = 0;
    let mut props = std::collections::HashMap::new();
    props.insert(
        "title".to_string(),
        zlf_core::Value::String(title.to_string()),
    );
    props.insert(
        "path".to_string(),
        zlf_core::Value::String(canonical.to_string_lossy().to_string()),
    );
    props.insert(
        "content".to_string(),
        zlf_core::Value::String(content.clone()),
    );
    props.insert(
        "source_type".to_string(),
        zlf_core::Value::String("markdown".to_string()),
    );

    let document = Node::with_id(
        doc_id.clone(),
        vec!["document".to_string(), "markdown".to_string()],
        props,
    );
    match db.add_node(document) {
        Ok(_) => count += 1,
        Err(e) => eprintln!(
            "Warning: Failed to import markdown document {}: {}",
            file.display(),
            e
        ),
    }

    for (index, chunk) in split_markdown_chunks(&content).into_iter().enumerate() {
        let chunk_id = format!("{}:chunk:{}", doc_id, index);
        let mut chunk_props = std::collections::HashMap::new();
        chunk_props.insert(
            "document_id".to_string(),
            zlf_core::Value::String(doc_id.clone()),
        );
        chunk_props.insert("index".to_string(), zlf_core::Value::Number(index as f64));
        chunk_props.insert("content".to_string(), zlf_core::Value::String(chunk));
        chunk_props.insert(
            "path".to_string(),
            zlf_core::Value::String(canonical.to_string_lossy().to_string()),
        );
        let chunk_node = Node::with_id(
            chunk_id.clone(),
            vec!["chunk".to_string(), "markdown_chunk".to_string()],
            chunk_props,
        );
        if db.add_node(chunk_node).is_ok() {
            count += 1;
        }

        let edge = Edge::with_id(
            format!("edge:{}:contains:{}", doc_id, index),
            "contains".to_string(),
            doc_id.clone(),
            chunk_id,
            std::collections::HashMap::new(),
        );
        if db.add_edge(edge).is_ok() {
            count += 1;
        }
    }

    Ok(count)
}

fn split_markdown_chunks(content: &str) -> Vec<String> {
    const MAX_CHARS: usize = 4000;
    let mut chunks = Vec::new();
    let mut current = String::new();

    for line in content.lines() {
        let starts_new_section = line.starts_with("# ") || line.starts_with("## ");
        if starts_new_section && !current.trim().is_empty() {
            chunks.push(current.trim().to_string());
            current.clear();
        }
        if current.len() + line.len() + 1 > MAX_CHARS && !current.trim().is_empty() {
            chunks.push(current.trim().to_string());
            current.clear();
        }
        current.push_str(line);
        current.push('\n');
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }
    if chunks.is_empty() && !content.trim().is_empty() {
        chunks.push(content.trim().to_string());
    }

    chunks
}

pub(crate) fn export_json(
    db: &ZlfDatabase,
    file: Option<&str>,
) -> Result<serde_json::Value, String> {
    let nodes = db.get_all_nodes().map_err(|e| e.to_string())?;
    let edges = db.get_all_edges().map_err(|e| e.to_string())?;
    let data = serde_json::json!({
        "nodes": nodes,
        "edges": edges
    });

    if let Some(file_path) = file {
        std::fs::write(file_path, serde_json::to_string_pretty(&data).unwrap())
            .map_err(|e| format!("Failed to write file: {}", e))?;
    }

    Ok(data)
}
