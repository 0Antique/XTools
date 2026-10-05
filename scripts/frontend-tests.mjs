import assert from 'node:assert/strict';
import ts from 'typescript';
import fs from 'node:fs';

async function sourceModule(file) {
  const source = fs.readFileSync(new URL(file, import.meta.url), 'utf8');
  const { outputText } = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } });
  return import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`);
}
const { adjacentCard } = await sourceModule('../src/lib/navigation.ts');
const { ContextQueue } = await sourceModule('../src/lib/rename-context.ts');
const card = (left, top, width = 100) => ({ left, top, width, height: 60 });
const cards = [card(0, 0), card(100, 0), card(200, 0), card(300, 0), card(0, 70), card(100, 70), card(0, 160, 140), card(150, 160, 140), card(300, 160, 140)];
assert.equal(adjacentCard(cards, 0, 'ArrowLeft'), 0);
assert.equal(adjacentCard(cards, 3, 'ArrowRight'), 3);
assert.equal(adjacentCard(cards, 3, 'ArrowDown'), 5); // incomplete recent row
assert.equal(adjacentCard(cards, 5, 'ArrowDown'), 7); // four columns to three
assert.equal(adjacentCard(cards, 8, 'ArrowUp'), 5);
assert.equal(adjacentCard(cards.slice(6), 0, 'ArrowRight'), 1);
assert.equal(adjacentCard([card(0, 0), card(0, 90), card(0, 180)], 1, 'ArrowLeft'), 1);
const queue = new ContextQueue();
assert(queue.receive({ requestId: 1, status: 'ready', paths: ['old'] }));
assert.equal(queue.take(true), undefined); // cannot replace an executing batch
assert(queue.receive({ requestId: 3, status: 'ready', paths: ['new'] }));
assert.equal(queue.receive({ requestId: 2, status: 'ready', paths: ['late'] }), false);
assert.deepEqual(queue.take(false).paths, ['new']);
assert.equal(queue.take(false), undefined); // exactly once
assert(queue.receive({ requestId: 4, status: 'empty', paths: [] }));
assert.equal(queue.take(false).status, 'empty');
console.log('Frontend navigation and invocation queue: 15 assertions passed');
