import { ZLF } from '../zlf';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

// Mock child_process to avoid slow cargo run calls
jest.mock('child_process', () => ({
  execFile: jest.fn(),
}));

// Mock util.promisify
jest.mock('util', () => ({
  promisify: jest.fn(() => {
    const mockExecFile = require('child_process').execFile;
    return (cmd: string, args: string[], options: any) => {
      return new Promise((resolve, reject) => {
        mockExecFile(cmd, args, options, (error: any, stdout: string, stderr: string) => {
          if (error) {
            reject(error);
          } else {
            resolve({ stdout, stderr });
          }
        });
      });
    };
  }),
}));

describe('ZLF TypeScript SDK', () => {
  let tempDir: string;
  let dbPath: string;
  let mockExecFile: jest.Mock;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'zlf-test-'));
    dbPath = path.join(tempDir, 'test-db');
    mockExecFile = require('child_process').execFile as jest.Mock;
    mockExecFile.mockReset();
  });

  afterEach(() => {
    if (fs.existsSync(tempDir)) {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('Constructor', () => {
    it('should create a ZLF instance with valid path', () => {
      const db = new ZLF(dbPath);
      expect(db).toBeDefined();
    });
  });

  describe('addNode', () => {
    it('should add a node and return it with ID', async () => {
      const mockResponse = {
        type: 'success',
        data: {
          id: 'test-node-id',
          labels: ['person'],
          properties: { name: 'Alice', age: 30 },
          current_version: 1,
          created_at: '2026-07-07T00:00:00Z',
          updated_at: '2026-07-07T00:00:00Z',
        },
      };
      mockExecFile.mockImplementation((cmd: string, args: string[], options: any, callback: any) => {
        callback(null, JSON.stringify(mockResponse), '');
      });

      const db = new ZLF(dbPath);
      const node = await db.addNode(['person'], { name: 'Alice', age: 30 });

      expect(node).toBeDefined();
      expect(node.id).toBe('test-node-id');
      expect(node.labels).toEqual(['person']);
      expect(node.properties.name).toBe('Alice');
    });

    it('should add node with empty labels', async () => {
      const mockResponse = {
        type: 'success',
        data: {
          id: 'test-node-id-2',
          labels: [],
          properties: { name: 'Test' },
          current_version: 1,
          created_at: '2026-07-07T00:00:00Z',
          updated_at: '2026-07-07T00:00:00Z',
        },
      };
      mockExecFile.mockImplementation((cmd: string, args: string[], options: any, callback: any) => {
        callback(null, JSON.stringify(mockResponse), '');
      });

      const db = new ZLF(dbPath);
      const node = await db.addNode([], { name: 'Test' });

      expect(node).toBeDefined();
      expect(node.labels).toEqual([]);
    });

    it('should add node with nested properties', async () => {
      const mockResponse = {
        type: 'success',
        data: {
          id: 'test-node-id-3',
          labels: ['person'],
          properties: { name: 'Alice', address: { city: 'Beijing', country: 'China' } },
          current_version: 1,
          created_at: '2026-07-07T00:00:00Z',
          updated_at: '2026-07-07T00:00:00Z',
        },
      };
      mockExecFile.mockImplementation((cmd: string, args: string[], options: any, callback: any) => {
        callback(null, JSON.stringify(mockResponse), '');
      });

      const db = new ZLF(dbPath);
      const node = await db.addNode(['person'], { 
        name: 'Alice', 
        address: { city: 'Beijing', country: 'China' } 
      });

      expect(node).toBeDefined();
      expect(node.properties.address.city).toBe('Beijing');
    });
  });

  describe('getNode', () => {
    it('should retrieve a node by ID', async () => {
      const mockResponse = {
        type: 'success',
        data: {
          id: 'test-node-id',
          labels: ['person'],
          properties: { name: 'Alice' },
          current_version: 1,
          created_at: '2026-07-07T00:00:00Z',
          updated_at: '2026-07-07T00:00:00Z',
        },
      };
      mockExecFile.mockImplementation((cmd: string, args: string[], options: any, callback: any) => {
        callback(null, JSON.stringify(mockResponse), '');
      });

      const db = new ZLF(dbPath);
      const retrieved = await db.getNode('test-node-id');

      expect(retrieved).toBeDefined();
      expect(retrieved?.id).toBe('test-node-id');
      expect(retrieved?.properties.name).toBe('Alice');
    });

    it('should return null for non-existent node', async () => {
      const mockResponse = {
        type: 'error',
        code: 'NODE_NOT_FOUND',
        message: 'Node nonexistent-id not found',
      };
      mockExecFile.mockImplementation((cmd: string, args: string[], options: any, callback: any) => {
        callback(null, JSON.stringify(mockResponse), '');
      });

      const db = new ZLF(dbPath);
      const retrieved = await db.getNode('nonexistent-id');

      expect(retrieved).toBeNull();
    });
  });

  describe('addEdge', () => {
    it('should add an edge between two nodes', async () => {
      const mockResponse = {
        type: 'success',
        data: {
          id: 'test-edge-id',
          edge_type: 'knows',
          source: 'node1',
          target: 'node2',
          properties: { since: 2020 },
          created_at: '2026-07-07T00:00:00Z',
          updated_at: '2026-07-07T00:00:00Z',
        },
      };
      mockExecFile.mockImplementation((cmd: string, args: string[], options: any, callback: any) => {
        callback(null, JSON.stringify(mockResponse), '');
      });

      const db = new ZLF(dbPath);
      const edge = await db.addEdge('knows', 'node1', 'node2', { since: 2020 });

      expect(edge).toBeDefined();
      expect(edge.id).toBe('test-edge-id');
      expect(edge.edge_type).toBe('knows');
      expect(edge.source).toBe('node1');
      expect(edge.target).toBe('node2');
      expect(edge.properties.since).toBe(2020);
    });

    it('should add edge with empty properties', async () => {
      const mockResponse = {
        type: 'success',
        data: {
          id: 'test-edge-id-2',
          edge_type: 'knows',
          source: 'node1',
          target: 'node2',
          properties: {},
          created_at: '2026-07-07T00:00:00Z',
          updated_at: '2026-07-07T00:00:00Z',
        },
      };
      mockExecFile.mockImplementation((cmd: string, args: string[], options: any, callback: any) => {
        callback(null, JSON.stringify(mockResponse), '');
      });

      const db = new ZLF(dbPath);
      const edge = await db.addEdge('knows', 'node1', 'node2');

      expect(edge).toBeDefined();
      expect(edge.properties).toEqual({});
    });
  });

  describe('getEdge', () => {
    it('should retrieve an edge by ID', async () => {
      const mockResponse = {
        type: 'success',
        data: {
          id: 'test-edge-id',
          edge_type: 'knows',
          source: 'node1',
          target: 'node2',
          properties: {},
          created_at: '2026-07-07T00:00:00Z',
          updated_at: '2026-07-07T00:00:00Z',
        },
      };
      mockExecFile.mockImplementation((cmd: string, args: string[], options: any, callback: any) => {
        callback(null, JSON.stringify(mockResponse), '');
      });

      const db = new ZLF(dbPath);
      const retrieved = await db.getEdge('test-edge-id');

      expect(retrieved).toBeDefined();
      expect(retrieved?.id).toBe('test-edge-id');
    });

    it('should return null for non-existent edge', async () => {
      const mockResponse = {
        type: 'error',
        code: 'EDGE_NOT_FOUND',
        message: 'Edge nonexistent-id not found',
      };
      mockExecFile.mockImplementation((cmd: string, args: string[], options: any, callback: any) => {
        callback(null, JSON.stringify(mockResponse), '');
      });

      const db = new ZLF(dbPath);
      const retrieved = await db.getEdge('nonexistent-id');

      expect(retrieved).toBeNull();
    });
  });

  describe('Memory operations', () => {
    it('should store and retrieve memory', async () => {
      const mockAddResponse = {
        type: 'success',
        data: {
          id: 'mem1',
          labels: ['memory', 'conversation'],
          properties: { content: { message: 'Hello' }, importance: 0.8 },
          current_version: 1,
          created_at: '2026-07-07T00:00:00Z',
          updated_at: '2026-07-07T00:00:00Z',
        },
      };
      const mockGetResponse = {
        type: 'success',
        data: {
          id: 'mem1',
          labels: ['memory', 'conversation'],
          properties: { content: { message: 'Hello' }, importance: 0.8 },
          current_version: 1,
          created_at: '2026-07-07T00:00:00Z',
          updated_at: '2026-07-07T00:00:00Z',
        },
      };
      mockExecFile
        .mockImplementationOnce((cmd: string, args: string[], options: any, callback: any) => {
          callback(null, JSON.stringify(mockAddResponse), '');
        })
        .mockImplementationOnce((cmd: string, args: string[], options: any, callback: any) => {
          callback(null, JSON.stringify(mockGetResponse), '');
        });

      const db = new ZLF(dbPath);
      
      await db.storeMemory('mem1', {
        type: 'conversation',
        content: { message: 'Hello' },
        entities: ['alice'],
        importance: 0.8,
      });
      
      const memory = await db.getMemory('mem1');

      expect(memory).toBeDefined();
      expect(memory?.type).toBe('conversation');
      expect(memory?.content.message).toBe('Hello');
    });

    it('should return null for non-existent memory', async () => {
      const mockResponse = {
        type: 'error',
        code: 'NODE_NOT_FOUND',
        message: 'Node nonexistent not found',
      };
      mockExecFile.mockImplementation((cmd: string, args: string[], options: any, callback: any) => {
        callback(null, JSON.stringify(mockResponse), '');
      });

      const db = new ZLF(dbPath);
      const memory = await db.getMemory('nonexistent');

      expect(memory).toBeNull();
    });
  });

  describe('Error handling', () => {
    it('should throw error for invalid database path', async () => {
      const mockResponse = {
        type: 'error',
        code: 'DB_OPEN_FAILED',
        message: 'Database not found: /nonexistent/path',
      };
      mockExecFile.mockImplementation((cmd: string, args: string[], options: any, callback: any) => {
        callback(null, JSON.stringify(mockResponse), '');
      });

      const db = new ZLF('/nonexistent/path');
      
      await expect(db.addNode(['test'], { name: 'test' }))
        .rejects.toThrow('[DB_OPEN_FAILED]');
    });

    it('should throw error for invalid JSON response', async () => {
      mockExecFile.mockImplementation((cmd: string, args: string[], options: any, callback: any) => {
        callback(null, 'invalid json', '');
      });

      const db = new ZLF(dbPath);
      
      await expect(db.addNode(['test'], { name: 'test' }))
        .rejects.toThrow();
    });
  });
});
