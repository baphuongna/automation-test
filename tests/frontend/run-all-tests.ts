import { readdirSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const testsDirectory = resolve("tests/frontend");
const testFiles = readdirSync(testsDirectory)
  .filter((fileName) => fileName.endsWith(".test.ts"))
  .sort((left, right) => left.localeCompare(right));

for (const fileName of testFiles) {
  await import(pathToFileURL(resolve(testsDirectory, fileName)).href);
}

console.log(`Executed ${testFiles.length} frontend regression tests.`);
