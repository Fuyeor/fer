// tools/vscode/test/grammar.test.mjs

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const directory = path.dirname(fileURLToPath(import.meta.url));
const grammar = JSON.parse(
  fs.readFileSync(path.join(directory, '..', 'syntaxes', 'fer.tmLanguage.json'), 'utf8'),
);
const manifest = JSON.parse(
  fs.readFileSync(path.join(directory, '..', 'package.json'), 'utf8'),
);

const functionRule = grammar.repository.functions.patterns.find((rule) => rule.name === 'entity.name.function.fer');
assert.ok(functionRule, 'function declaration rule must exist');
const functionPattern = new RegExp(functionRule.match);
assert.equal(functionPattern.test('main = () -> i64 { 42 }'), true);
assert.equal(functionPattern.test('hello-world = () {}'), true);
assert.equal(functionPattern.test('Main = () {}'), false);
assert.equal(functionPattern.test('main() {}'), false);

const keywordPattern = grammar.repository.keywords.patterns[0].match;
for (const keyword of ['all', 'any', 'one', 'none']) {
  assert.match(keywordPattern, new RegExp(`\\b${keyword}\\b`));
}
const logicalPattern = grammar.repository.operators.patterns[0].match;
assert.equal(/\b(?:and|or)\b/.test(logicalPattern), false);
assert.equal(manifest.categories.includes('Formatters'), false);
assert.equal(manifest.main, './out/extension.js');
