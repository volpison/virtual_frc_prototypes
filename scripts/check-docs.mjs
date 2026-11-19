// Checks the documentation so it cannot quietly rot. No dependencies (Node 20 or newer).
//
//   node scripts/check-docs.mjs
//
// It fails (exit code 1) when:
//   - a relative link in README.md, ROADMAP.md or docs/ points at a file that does not exist,
//   - a link with #heading points at a heading that does not exist,
//   - docs/index.md does not list a page under docs/.
//
// The checking functions are exported and tested in scripts/check-docs.test.mjs.
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";

/** Every .md file under a folder (empty when the folder does not exist). */
export function markdownFiles(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? markdownFiles(path) : name.endsWith(".md") ? [path] : [];
  });
}

/** The anchors GitHub makes from the headings of a Markdown text. */
export function anchorsOf(text) {
  return new Set(
    text
      .replace(/```[\s\S]*?```/g, "")
      .split("\n")
      .filter((line) => /^#{1,6} /.test(line))
      .map((line) =>
        line
          .replace(/^#+ /, "")
          .toLowerCase()
          .replace(/[`*_]/g, "")
          .replace(/[^\p{L}\p{N} -]/gu, "")
          .trim()
          .replace(/ /g, "-")
      )
  );
}

const forwardSlashes = (p) => p.split(sep).join("/");

/** Problems with the links of one file ("README.md: docs/nope.md"). `root` is the project folder. */
export function linkProblems(root, file) {
  const problems = [];
  const name = forwardSlashes(relative(root, file));
  const text = readFileSync(file, "utf8").replace(/```[\s\S]*?```/g, "");
  for (const match of text.matchAll(/\]\(([^)\s]+)\)/g)) {
    const link = match[1];
    if (/^(https?:|mailto:)/.test(link)) continue;
    const [path, hash] = link.split("#");
    const target = path ? resolve(dirname(file), path) : file;
    if (!existsSync(target)) {
      problems.push(`${name}: ${link} (no such file)`);
    } else if (hash && target.endsWith(".md") && !anchorsOf(readFileSync(target, "utf8")).has(hash)) {
      problems.push(`${name}: ${link} (no such heading)`);
    }
  }
  return problems;
}

/** Pages under docs/ that docs/index.md does not link to. */
export function unlistedPages(root) {
  const docs = join(root, "docs");
  const indexPath = join(docs, "index.md");
  if (!existsSync(indexPath)) return markdownFiles(docs).length ? ["docs/index.md is missing"] : [];
  const index = readFileSync(indexPath, "utf8");
  return markdownFiles(docs)
    .map((p) => forwardSlashes(relative(docs, p)))
    .filter((p) => p !== "index.md" && !index.includes(`(${p})`))
    .map((p) => `docs/index.md does not list ${p}`);
}

/** All problems of a project folder. */
export function docsProblems(root) {
  const files = [join(root, "README.md"), join(root, "ROADMAP.md")].filter(existsSync).concat(markdownFiles(join(root, "docs")));
  return [...files.flatMap((f) => linkProblems(root, f)), ...unlistedPages(root)];
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const problems = docsProblems(resolve(import.meta.dirname, ".."));
  if (problems.length) {
    console.error(problems.join("\n"));
    console.error(`\n${problems.length} documentation problem${problems.length === 1 ? "" : "s"}.`);
    process.exit(1);
  }
  console.log("Documentation links are fine.");
}
