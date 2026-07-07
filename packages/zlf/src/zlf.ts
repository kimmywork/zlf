import { execFile } from 'child_process';
import { promisify } from 'util';
import * as path from 'path';
import { Node, Edge, QueryResult, SearchResult, SimilarResult, MemoryData } from './types';

const execFileAsync = promisify(execFile);

interface ZLFResponse {
  type: 'success' | 'error';
  data?: any;
  code?: string;
  message?: string;
}

export class ZLF {
  private dbPath: string;
  private zlfBinary: string;

  constructor(dbPath: string) {
    this.dbPath = dbPath;
    // Find the zlf binary - look in cargo target directory
    this.zlfBinary = this.findZlfBinary();
  }

  private findZlfBinary(): string {
    // Try to find the zlf binary in the project
    const possiblePaths = [
      path.join(process.cwd(), 'target', 'release', 'zlf'),
      path.join(process.cwd(), 'target', 'debug', 'zlf'),
      '/usr/local/bin/zlf',
      '/usr/bin/zlf',
    ];

    for (const p of possiblePaths) {
      try {
        const fs = require('fs');
        if (fs.existsSync(p)) {
          return p;
        }
      } catch {}
    }

    // Default to cargo run (will be slow on first run)
    return 'cargo';
  }

  private async executeCommand(command: object): Promise<any> {
    const request = JSON.stringify(command);
    
    let result: { stdout: string; stderr: string };
    
    if (this.zlfBinary.includes('cargo')) {
      // Use cargo run (slower, compiles on first run)
      result = await execFileAsync('cargo', ['run', '-p', 'zlf-cli', '--release', '--', request], {
        maxBuffer: 10 * 1024 * 1024, // 10MB
        timeout: 60000, // 60 seconds timeout
      });
    } else {
      // Use the binary directly (fast)
      result = await execFileAsync(this.zlfBinary, [request], {
        maxBuffer: 10 * 1024 * 1024, // 10MB
        timeout: 10000, // 10 seconds timeout
      });
    }

    const response: ZLFResponse = JSON.parse(result.stdout.trim());
    
    if (response.type === 'error') {
      throw new Error(`[${response.code}] ${response.message}`);
    }
    
    return response.data;
  }

  async addNode(labels: string[], properties: Record<string, any>): Promise<Node> {
    return this.executeCommand({
      command: 'add_node',
      path: this.dbPath,
      labels,
      properties,
    });
  }

  async getNode(id: string): Promise<Node | null> {
    try {
      return await this.executeCommand({
        command: 'get_node',
        path: this.dbPath,
        id,
      });
    } catch (error: any) {
      if (error.message.includes('NODE_NOT_FOUND')) {
        return null;
      }
      throw error;
    }
  }

  async addEdge(type: string, source: string, target: string, properties: Record<string, any> = {}): Promise<Edge> {
    return this.executeCommand({
      command: 'add_edge',
      path: this.dbPath,
      edge_type: type,
      source,
      target,
      properties,
    });
  }

  async getEdge(id: string): Promise<Edge | null> {
    try {
      return await this.executeCommand({
        command: 'get_edge',
        path: this.dbPath,
        id,
      });
    } catch (error: any) {
      if (error.message.includes('EDGE_NOT_FOUND')) {
        return null;
      }
      throw error;
    }
  }

  async query(queryStr: string): Promise<QueryResult[]> {
    return this.executeCommand({
      command: 'query',
      path: this.dbPath,
      query: queryStr,
    });
  }

  async search(query: string): Promise<SearchResult[]> {
    return this.executeCommand({
      command: 'search',
      path: this.dbPath,
      query,
    });
  }

  async similar(nodeId: string, threshold: number = 0.8, limit: number = 10): Promise<SimilarResult[]> {
    return this.executeCommand({
      command: 'similar',
      path: this.dbPath,
      node_id: nodeId,
      threshold,
      limit,
    });
  }

  // Memory operations (high-level wrapper)
  async storeMemory(id: string, data: MemoryData): Promise<void> {
    // Store as a node with memory labels
    const labels = ['memory', data.type || 'unknown'];
    const properties = {
      content: data.content,
      entities: data.entities || [],
      topics: data.topics || [],
      importance: data.importance || 0.5,
      ttl: data.ttl,
    };
    await this.addNode(labels, properties);
  }

  async getMemory(id: string): Promise<MemoryData | null> {
    const node = await this.getNode(id);
    if (!node) return null;
    
    return {
      type: node.labels.find(l => l !== 'memory') || 'unknown',
      content: node.properties.content || {},
      entities: node.properties.entities || [],
      topics: node.properties.topics || [],
      importance: node.properties.importance || 0.5,
      ttl: node.properties.ttl,
    };
  }

  async queryMemories(pattern: {
    type?: string;
    entities?: string[];
    topics?: string[];
  }): Promise<MemoryData[]> {
    // Build a query based on pattern
    let query = 'node(memory, X, Props)';
    const conditions: string[] = [];
    
    if (pattern.type) {
      conditions.push(`node(${pattern.type}, X, _)`);
    }
    
    if (conditions.length > 0) {
      query += ', ' + conditions.join(', ');
    }
    
    const results = await this.query(query);
    return results.map(r => ({
      type: r.labels?.find((l: string) => l !== 'memory') || 'unknown',
      content: r.properties?.content || {},
      entities: r.properties?.entities || [],
      topics: r.properties?.topics || [],
      importance: r.properties?.importance || 0.5,
      ttl: r.properties?.ttl,
    }));
  }
}
