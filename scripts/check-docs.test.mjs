import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { anchorsOf, docsProblems } from "./check-docs.mjs";

function project(files) {
  const root = mkdtempSync(join(tmpdir(), "docs-check-"));
  for (const [path, text] of Object.entries(files)) {
    const full = join(root, path);
    mkdirSync(join(full, ".."), { recursive: true });
    writeFileSync(full, text);
  }
  return root;
}

test("anchors are made like GitHub makes them", () => {
  const a = anchorsOf("# Hello, World!\n## `Code` and *style*\n```\n# not a heading\n```\n");
  assert.deepEqual([...a], ["hello-world", "code-and-style"]);
});

test("a healthy project has no problems", () => {
  const root = project({
    "README.md": "See [the docs](docs/index.md) and [a heading](docs/a.md#first-part) and [web](https://example.com).",
    "docs/index.md": "- [A](a.md)\n",
    "docs/a.md": "# A\n## First part\n",
  });
  assert.deepEqual(docsProblems(root), []);
});

test("broken file links, broken headings and unlisted pages are reported", () => {
  const root = project({
    "README.md": "[gone](docs/nope.md) and [bad](docs/a.md#missing)",
    "docs/index.md": "- [A](a.md)\n",
    "docs/a.md": "# A\n",
    "docs/b.md": "# B\n",
  });
  assert.deepEqual(docsProblems(root), [
    "README.md: docs/nope.md (no such file)",
    "README.md: docs/a.md#missing (no such heading)",
    "docs/index.md does not list b.md",
  ]);
});

test("links inside code blocks are ignored, and a project without docs is fine", () => {
  const root = project({ "README.md": "```\n[x](nowhere.md)\n```\n" });
  assert.deepEqual(docsProblems(root), []);
});
