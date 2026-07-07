export interface Node {
  id: string;
  labels: string[];
  properties: Record<string, any>;
  current_version: number;
  created_at: string;
  updated_at: string;
}

export interface Edge {
  id: string;
  edge_type: string;
  source: string;
  target: string;
  properties: Record<string, any>;
  created_at: string;
  updated_at: string;
}

export interface QueryResult {
  [key: string]: any;
}

export interface SearchResult {
  node_id: string;
  score: number;
}

export interface SimilarResult {
  node_id: string;
  similarity: number;
}

export interface MemoryData {
  type: string;
  content: Record<string, any>;
  entities?: string[];
  topics?: string[];
  importance?: number;
  ttl?: number;
}
