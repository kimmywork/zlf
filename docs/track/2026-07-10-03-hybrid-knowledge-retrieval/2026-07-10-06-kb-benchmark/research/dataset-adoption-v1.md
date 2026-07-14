# Stage 06 public dataset adoption v1

Accessed 2026-07-14. Raw datasets remain local and are not redistributed by zlf.

## FiQA — adopted with license caution

**Evidence:**

- BEIR dataset inventory identifies FiQA-2018 as a 57K-corpus question-answering retrieval task and publishes archive checksum `17918ed23cd04fb15047f73e6c3bd9d9`: <https://github.com/beir-cellar/beir/wiki/Datasets-available>.
- Hugging Face `BeIR/fiqa` reports CC-BY-SA-4.0 and provides corpus/query parquet files: <https://huggingface.co/datasets/BeIR/fiqa>.
- BEIR explicitly states that users remain responsible for determining source-dataset permissions.

**Decision:** adopt for local evaluation, preserve attribution and checksums, do not commit/redistribute source rows, and retain `pending_upstream_review` in machine-readable reports rather than treating the secondary BEIR card as definitive original-source licensing.

## MIRACL English/Chinese — adopted as bounded judged pools

**Evidence:**

- Official repository describes native-speaker topics/judgments, corpus passage schema, dev counts, and links to Hugging Face sources: <https://github.com/project-miracl/miracl>.
- Repository and Hugging Face dataset report Apache-2.0 for MIRACL artifacts: <https://huggingface.co/datasets/miracl/miracl>.
- Corpus is derived from Wikipedia dumps and therefore carries source-content attribution/terms beyond the benchmark code/annotation license.

**Decision:** adopt local en/zh shard-0 judged pools. Preserve official qrels unchanged, retain only queries whose complete positive set is present, label reports as non-leaderboard subset evidence, do not redistribute corpus rows, and retain `pending_upstream_review` with Apache-2.0 annotation and Wikipedia-origin notes.

## HotpotQA/KILT — deferred pending focused multi-hop mapping

No data is downloaded in this increment. Adoption requires a direct graph/text supporting-fact mapping that measures zlf multi-hop retrieval rather than LLM answer generation, plus primary-source license confirmation. This does not block FiQA/MIRACL quality acceptance.

## LoCoMo/LongMemEval — not adopted

These primarily evaluate end-to-end long-memory question answering. zlf currently has no LLM answer-generation, prompt assembly, or answer-judging path. Converting them into custom retrieval judgments would not be comparable to published metrics. Revisit only after an LLM/RAG answer pipeline exists or official retrieval judgments become available.

## Confidence

- MIRACL schema, qrels format, source links, and repository license: **confirmed** from official repository/Hugging Face sources.
- FiQA BEIR schema/checksum and Hugging Face reported license: **confirmed** as distributor metadata; original-source permission remains **unconfirmed**, hence the explicit caution.
- Agent-memory non-adoption rationale: **confirmed** against current zlf scope and runtime architecture.
