# Change Note v3: Pin first Tantivy BM25 parameters

**Date:** 2026-07-11  
**Status:** accepted for first functional backend

## Change

The first production BM25 backend accepts the versioned Tantivy defaults only:

- `k1 = 1.2`
- `b = 0.75`
- analyzer `unicode_jieba_v1`, version `1`

Profiles remain explicit and serialize all values, but profile validation rejects unsupported analyzer versions or non-default `k1`/`b` instead of silently producing scores with different settings.

## Rationale

Tantivy 0.22 fixes `k1` and `b` in its BM25 scorer. Replacing its scorer or re-ranking an unbounded corpus solely to make these values tunable would work against the function-first decision to use a mainstream, bounded, stable backend. The independent oracle remains parameterized so a later custom scorer can be verified without changing the public profile shape.

## Requirement delta

Stage 02's initial requirement for configurable `k1` and `b` is narrowed to **explicit, versioned, and compatibility-validated** parameters for the first Tantivy generation. Supporting additional parameter combinations is a post-functional extension and must not be implemented as approximate candidate re-ranking.
