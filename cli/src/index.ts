#!/usr/bin/env node

import { Command } from 'commander';
import { ZLF } from 'zlf';
import * as fs from 'fs';
import * as path from 'path';

const program = new Command();

program
  .name('zlf')
  .description('CLI for zlf graph database')
  .version('0.1.0');

program
  .command('init')
  .description('Initialize a new database')
  .argument('[path]', 'Database path', './zlf-db')
  .action(async (dbPath: string) => {
    try {
      if (fs.existsSync(dbPath)) {
        console.error(`Error: Database already exists at ${dbPath}`);
        process.exit(1);
      }
      
      fs.mkdirSync(dbPath, { recursive: true });
      const db = new ZLF(dbPath);
      console.log(`Database initialized at ${dbPath}`);
    } catch (error) {
      console.error(`Error: ${error}`);
      process.exit(1);
    }
  });

program
  .command('query')
  .description('Execute a query')
  .argument('<query>', 'The query string')
  .option('-d, --db <path>', 'Database path', './zlf-db')
  .action(async (query: string, options: { db: string }) => {
    try {
      const db = new ZLF(options.db);
      const results = await db.query(query);
      console.log(JSON.stringify(results, null, 2));
    } catch (error) {
      console.error(`Error: ${error}`);
      process.exit(1);
    }
  });

program
  .command('node-add')
  .description('Add a node')
  .argument('<labels>', 'Node labels (comma-separated)')
  .argument('<properties>', 'Node properties (JSON)')
  .option('-d, --db <path>', 'Database path', './zlf-db')
  .action(async (labels: string, properties: string, options: { db: string }) => {
    try {
      const db = new ZLF(options.db);
      const labelList = labels.split(',').map(l => l.trim());
      const props = JSON.parse(properties);
      const node = await db.addNode(labelList, props);
      console.log(`Node created: ${node.id}`);
    } catch (error) {
      console.error(`Error: ${error}`);
      process.exit(1);
    }
  });

program
  .command('node-get')
  .description('Get a node by ID')
  .argument('<id>', 'Node ID')
  .option('-d, --db <path>', 'Database path', './zlf-db')
  .action(async (id: string, options: { db: string }) => {
    try {
      const db = new ZLF(options.db);
      const node = await db.getNode(id);
      if (node) {
        console.log(JSON.stringify(node, null, 2));
      } else {
        console.error(`Node not found: ${id}`);
        process.exit(1);
      }
    } catch (error) {
      console.error(`Error: ${error}`);
      process.exit(1);
    }
  });

program
  .command('edge-add')
  .description('Add an edge')
  .argument('<type>', 'Edge type')
  .argument('<source>', 'Source node ID')
  .argument('<target>', 'Target node ID')
  .argument('[properties]', 'Edge properties (JSON)', '{}')
  .option('-d, --db <path>', 'Database path', './zlf-db')
  .action(async (type: string, source: string, target: string, properties: string, options: { db: string }) => {
    try {
      const db = new ZLF(options.db);
      const props = JSON.parse(properties);
      const edge = await db.addEdge(type, source, target, props);
      console.log(`Edge created: ${edge.id}`);
    } catch (error) {
      console.error(`Error: ${error}`);
      process.exit(1);
    }
  });

program
  .command('search')
  .description('Search by text')
  .argument('<query>', 'Search query')
  .option('-d, --db <path>', 'Database path', './zlf-db')
  .action(async (query: string, options: { db: string }) => {
    try {
      const db = new ZLF(options.db);
      const results = await db.search(query);
      if (results.length === 0) {
        console.log('No results found');
      } else {
        results.forEach(r => {
          console.log(`${r.node_id}: ${r.score}`);
        });
      }
    } catch (error) {
      console.error(`Error: ${error}`);
      process.exit(1);
    }
  });

program
  .command('similar')
  .description('Find similar nodes')
  .argument('<node-id>', 'Node ID')
  .option('-t, --threshold <number>', 'Similarity threshold', '0.8')
  .option('-d, --db <path>', 'Database path', './zlf-db')
  .action(async (nodeId: string, options: { threshold: string; db: string }) => {
    try {
      const db = new ZLF(options.db);
      const threshold = parseFloat(options.threshold);
      const results = await db.similar(nodeId, threshold);
      if (results.length === 0) {
        console.log('No similar nodes found');
      } else {
        results.forEach(r => {
          console.log(`${r.node_id}: ${r.similarity}`);
        });
      }
    } catch (error) {
      console.error(`Error: ${error}`);
      process.exit(1);
    }
  });

program.parse();
