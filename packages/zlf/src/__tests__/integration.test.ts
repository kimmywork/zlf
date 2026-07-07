import { ZLF } from '../zlf';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

// Integration tests - these test the actual TypeScript SDK → Rust CLI flow
// They require the Rust binary to be built first

// Check if the Rust binary exists (look in project root)
const projectRoot = path.resolve(process.cwd(), '../..');
const rustBinary = path.join(projectRoot, 'target', 'release', 'zlf');
const hasRustBinary = fs.existsSync(rustBinary);

// Skip integration tests if binary not found
const describeIfBinary = hasRustBinary ? describe : describe.skip;

// Increase timeout for integration tests that call Rust CLI
jest.setTimeout(60000);

describeIfBinary('ZLF TypeScript SDK Integration Tests', () => {
  let tempDir: string;
  let dbPath: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'zlf-integration-test-'));
    dbPath = path.join(tempDir, 'test-db');
  });

  afterEach(() => {
    if (fs.existsSync(tempDir)) {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('Full Flow Integration', () => {
    it('should initialize database and add nodes', async () => {
      const db = new ZLF(dbPath);
      
      // Add a node
      const node = await db.addNode(['person'], { name: 'Alice', age: 30 });
      
      expect(node).toBeDefined();
      expect(node.id).toBeDefined();
      expect(node.labels).toEqual(['person']);
      expect(node.properties.name).toBe('Alice');
      expect(node.properties.age).toBe(30);
    });

    it('should add and retrieve nodes', async () => {
      const db = new ZLF(dbPath);
      
      // Add node
      const node = await db.addNode(['person'], { name: 'Bob' });
      
      // Retrieve node
      const retrieved = await db.getNode(node.id);
      
      expect(retrieved).toBeDefined();
      expect(retrieved?.id).toBe(node.id);
      expect(retrieved?.properties.name).toBe('Bob');
    });

    it('should add edges between nodes', async () => {
      const db = new ZLF(dbPath);
      
      // Add nodes
      const node1 = await db.addNode(['person'], { name: 'Alice' });
      const node2 = await db.addNode(['person'], { name: 'Bob' });
      
      // Add edge
      const edge = await db.addEdge('knows', node1.id, node2.id, { since: 2020 });
      
      expect(edge).toBeDefined();
      expect(edge.edge_type).toBe('knows');
      expect(edge.source).toBe(node1.id);
      expect(edge.target).toBe(node2.id);
    });

    it('should handle memory operations', async () => {
      const db = new ZLF(dbPath);
      
      // Store memory
      await db.storeMemory('mem1', {
        type: 'conversation',
        content: { message: 'Hello World' },
        entities: ['alice', 'bob'],
        importance: 0.9,
      });
      
      // Retrieve memory
      const memory = await db.getMemory('mem1');
      
      expect(memory).toBeDefined();
      expect(memory?.type).toBe('conversation');
      expect(memory?.content.message).toBe('Hello World');
    });
  });

  describe('Edge Cases', () => {
    it('should handle node with empty labels', async () => {
      const db = new ZLF(dbPath);
      
      const node = await db.addNode([], { name: 'Test' });
      
      expect(node).toBeDefined();
      expect(node.labels).toEqual([]);
    });

    it('should handle node with empty properties', async () => {
      const db = new ZLF(dbPath);
      
      const node = await db.addNode(['test'], {});
      
      expect(node).toBeDefined();
      expect(Object.keys(node.properties)).toHaveLength(0);
    });

    it('should handle node with nested properties', async () => {
      const db = new ZLF(dbPath);
      
      const node = await db.addNode(['person'], {
        name: 'Alice',
        address: {
          city: 'Beijing',
          country: 'China',
          coordinates: { lat: 39.9, lng: 116.4 }
        }
      });
      
      expect(node).toBeDefined();
      expect(node.properties.address.city).toBe('Beijing');
      expect(node.properties.address.coordinates.lat).toBe(39.9);
    });

    it('should handle node with large properties (>1KB)', async () => {
      const db = new ZLF(dbPath);
      
      const largeContent = 'x'.repeat(2000);
      const node = await db.addNode(['document'], { content: largeContent });
      
      expect(node).toBeDefined();
      expect(node.properties.content).toBe(largeContent);
    });

    it('should handle node with special characters in properties', async () => {
      const db = new ZLF(dbPath);
      
      const node = await db.addNode(['person'], {
        name: 'Alice Smith',
        bio: 'Software engineer with 10+ years experience',
        emoji: '🎉',
        chinese: '你好世界'
      });
      
      expect(node).toBeDefined();
      expect(node.properties.emoji).toBe('🎉');
      expect(node.properties.chinese).toBe('你好世界');
    });

    it('should handle edge with no properties', async () => {
      const db = new ZLF(dbPath);
      
      const node1 = await db.addNode(['person'], { name: 'Alice' });
      const node2 = await db.addNode(['person'], { name: 'Bob' });
      
      const edge = await db.addEdge('knows', node1.id, node2.id);
      
      expect(edge).toBeDefined();
      expect(edge.properties).toEqual({});
    });

    it('should handle self-referencing edge', async () => {
      const db = new ZLF(dbPath);
      
      const node = await db.addNode(['person'], { name: 'Alice' });
      
      const edge = await db.addEdge('knows', node.id, node.id);
      
      expect(edge).toBeDefined();
      expect(edge.source).toBe(node.id);
      expect(edge.target).toBe(node.id);
    });
  });

  describe('Unhappy Paths', () => {
    it('should return null for non-existent node', async () => {
      const db = new ZLF(dbPath);
      
      const node = await db.getNode('nonexistent-id');
      
      expect(node).toBeNull();
    });

    it('should return null for non-existent edge', async () => {
      const db = new ZLF(dbPath);
      
      const edge = await db.getEdge('nonexistent-id');
      
      expect(edge).toBeNull();
    });

    it('should return null for non-existent memory', async () => {
      const db = new ZLF(dbPath);
      
      const memory = await db.getMemory('nonexistent');
      
      expect(memory).toBeNull();
    });

    it('should throw error for invalid database path', async () => {
      const db = new ZLF('/nonexistent/path');
      
      await expect(db.addNode(['test'], { name: 'test' }))
        .rejects.toThrow();
    });
  });

  describe('Multiple Operations', () => {
    it('should handle multiple sequential operations', async () => {
      const db = new ZLF(dbPath);
      
      // Add multiple nodes
      const nodes = [];
      for (let i = 0; i < 5; i++) {
        const node = await db.addNode(['person'], { name: `User${i}` });
        nodes.push(node);
      }
      
      // Verify all nodes were created
      expect(nodes).toHaveLength(5);
      
      // Retrieve each node
      for (const node of nodes) {
        const retrieved = await db.getNode(node.id);
        expect(retrieved).toBeDefined();
        expect(retrieved?.id).toBe(node.id);
      }
    });

    it('should handle complex graph structure', async () => {
      const db = new ZLF(dbPath);
      
      // Create nodes
      const alice = await db.addNode(['person'], { name: 'Alice' });
      const bob = await db.addNode(['person'], { name: 'Bob' });
      const charlie = await db.addNode(['person'], { name: 'Charlie' });
      const acme = await db.addNode(['company'], { name: 'ACME' });
      
      // Create edges
      await db.addEdge('knows', alice.id, bob.id);
      await db.addEdge('knows', bob.id, charlie.id);
      await db.addEdge('works_at', bob.id, acme.id);
      await db.addEdge('works_at', charlie.id, acme.id);
      
      // Verify nodes exist
      expect(await db.getNode(alice.id)).toBeDefined();
      expect(await db.getNode(bob.id)).toBeDefined();
      expect(await db.getNode(charlie.id)).toBeDefined();
      expect(await db.getNode(acme.id)).toBeDefined();
    });
  });
});
