use std::path::Path;

use hnsw_rs::prelude::{AnnT, DistCosine, Hnsw, HnswIo};
use serde_json::{json, Value};

const BASENAME: &str = "lifecycle";

#[allow(clippy::too_many_lines)]
pub fn probe(parent: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir_in(parent)?;
    let original = vectors(&[
        (vec![1.0, 0.0, 0.0, 0.0], 0),
        (vec![0.0, 1.0, 0.0, 0.0], 1),
        (vec![-1.0, 0.0, 0.0, 0.0], 2),
    ]);
    let before = build(&original);
    let before_top1 = top1(&before, &[1.0, 0.0, 0.0, 0.0]);

    // hnsw_rs has no delete/update API. Publish a replacement immutable generation.
    let rebuilt = vectors(&[
        (vec![0.0, 1.0, 0.0, 0.0], 0),
        (vec![-1.0, 0.0, 0.0, 0.0], 2),
        (vec![1.0, 0.0, 0.0, 0.0], 3),
    ]);
    let after = build(&rebuilt);
    let updated_top1 = top1(&after, &[0.0, 1.0, 0.0, 0.0]);
    let inserted_top1 = top1(&after, &[1.0, 0.0, 0.0, 0.0]);
    let deleted_absent = after
        .search(&[0.0, 1.0, 0.0, 0.0], rebuilt.len(), 32)
        .iter()
        .all(|hit| hit.d_id != 1);
    after.file_dump(directory.path(), BASENAME)?;
    drop(after);

    let mut io = HnswIo::new(directory.path(), BASENAME);
    let reopened: Hnsw<f32, DistCosine> = io.load_hnsw()?;
    let reopen_updated_top1 = top1(&reopened, &[0.0, 1.0, 0.0, 0.0]);
    let reopen_inserted_top1 = top1(&reopened, &[1.0, 0.0, 0.0, 0.0]);
    Ok(json!({
        "policy":"immutable_generation_rebuild",
        "in_place_update_supported":false,
        "in_place_delete_supported":false,
        "before_top1":before_top1,
        "updated_top1":updated_top1,
        "deleted_absent":deleted_absent,
        "inserted_top1":inserted_top1,
        "reopen_updated_top1":reopen_updated_top1,
        "reopen_inserted_top1":reopen_inserted_top1,
        "passed":before_top1 == 0 && updated_top1 == 0 && deleted_absent
            && inserted_top1 == 3 && reopen_updated_top1 == 0 && reopen_inserted_top1 == 3,
    }))
}

fn build<'a>(vectors: &'a [(Vec<f32>, usize)]) -> Hnsw<'a, f32, DistCosine> {
    let mut index = Hnsw::new(8, vectors.len(), 16, 32, DistCosine {});
    let references = vectors
        .iter()
        .map(|(values, id)| (values, *id))
        .collect::<Vec<_>>();
    index.parallel_insert(&references);
    index.set_searching_mode(true);
    index
}

fn vectors(entries: &[(Vec<f32>, usize)]) -> Vec<(Vec<f32>, usize)> {
    entries.to_vec()
}

fn top1(index: &Hnsw<f32, DistCosine>, query: &[f32]) -> usize {
    index.search(query, 1, 32)[0].d_id
}
