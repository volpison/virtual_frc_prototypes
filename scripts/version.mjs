// One version number, written in several files. This keeps them equal and checks a release tag.
//
//   node scripts/version.mjs show                 print the version found in every configured file
//   node scripts/version.mjs set 1.2.0            change all of them
//   node scripts/version.mjs check                fail unless they are all equal
//   node scripts/version.mjs check v1.2.0         fail unless they all equal the tag (the release workflow runs this)
//
// Which files hold the version is listed in version-files.json (see docs/contributing.md).
// No dependencies (Node 20 or newer). The functions are exported and tested in scripts/version.test.mjs.
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const isVersion = (v) => typeof v === "string" && /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(v);

const escapeRegExp = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/**
 * The regular expression that finds the version in a file, as [before, version, after] groups.
 *   json   the first "version": "x" in the file (package.json, tauri.conf.json)
 *   toml   `key = "x"` inside [section] (Cargo.toml: package, pyproject.toml: project)
 *   regex  your own pattern with three groups: (text before)(the version)(text after)
 */
export function matcherFor(entry) {
  switch (entry.kind) {
    case "json":
      return /("version"\s*:\s*")([^"]+)(")/;
    case "toml": {
      const section = escapeRegExp(entry.section ?? "package");
      const key = escapeRegExp(entry.key ?? "version");
      return new RegExp(`(^\\[${section}\\][^\\[]*?^${key}\\s*=\\s*")([^"]+)(")`, "m");
    }
    case "regex":
      return new RegExp(entry.pattern, "m");
    default:
      throw new Error(`${entry.file}: unknown kind "${entry.kind}" (use json, toml or regex).`);
  }
}

/** Every configured file's version: { file: version | null }. `read(file)` gives a file's text, or null if it is missing. */
export function readVersions(files, read) {
  const out = {};
  for (const entry of files) {
    const text = read(entry.file);
    out[entry.file] = text === null ? null : (matcherFor(entry).exec(text)?.[2] ?? null);
  }
  return out;
}

/** Problems with releasing `tag` ("v1.2.0"), or with the versions being unequal when no tag is given. Empty = fine. */
export function problems(versions, tag) {
  const out = [];
  const wanted = tag === undefined ? undefined : tag.replace(/^v/, "");
  if (tag !== undefined && (!/^v/.test(tag) || !isVersion(wanted))) return [`The tag "${tag}" is not like v1.2.0.`];
  const entries = Object.entries(versions);
  if (entries.length === 0) return ["No version files are configured: list them in version-files.json."];
  const reference = wanted ?? entries.find(([, v]) => v !== null)?.[1];
  for (const [file, found] of entries) {
    if (found === null) out.push(`${file}: no version found.`);
    else if (found !== reference) out.push(wanted === undefined ? `${file} says ${found}, but others say ${reference}.` : `${file} says ${found}, but the tag is ${tag}.`);
  }
  return out;
}

/** Write `version` into every configured file. `read` as above; `write(file, text)`. Returns the files changed. */
export function setVersions(files, version, read, write) {
  if (!isVersion(version)) throw new Error(`"${version}" is not a version like 1.2.0.`);
  const changed = [];
  for (const entry of files) {
    const text = read(entry.file);
    if (text === null) throw new Error(`${entry.file}: no such file.`);
    const re = matcherFor(entry);
    if (!re.test(text)) throw new Error(`${entry.file}: no version found.`);
    // Edited as text, never re-serialised: line endings and every other line stay exactly as they were.
    write(entry.file, text.replace(re, (_, before, _old, after) => `${before}${version}${after}`));
    changed.push(entry.file);
  }
  return changed;
}

export function loadConfig(readFile = (f) => (existsSync(f) ? readFileSync(f, "utf8") : null)) {
  const text = readFile("version-files.json");
  if (text === null) throw new Error("version-files.json is missing.");
  const config = JSON.parse(text);
  if (!Array.isArray(config.files)) throw new Error('version-files.json needs a "files" list.');
  return config.files;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.chdir(resolve(import.meta.dirname, ".."));
  const readFile = (f) => (existsSync(f) ? readFileSync(f, "utf8") : null);
  const [command, arg] = process.argv.slice(2);
  try {
    const files = loadConfig(readFile);
    if (command === "show") {
      for (const [file, v] of Object.entries(readVersions(files, readFile))) console.log(`${file}: ${v ?? "(none)"}`);
    } else if (command === "check") {
      const found = problems(readVersions(files, readFile), arg);
      if (found.length) {
        console.error(found.join("\n"));
        if (files.length) console.error("\nChange them together with:  node scripts/version.mjs set <x.y.z>");
        process.exit(1);
      }
      console.log(arg ? `All versions match ${arg}.` : "All versions are equal.");
    } else if (command === "set") {
      const changed = setVersions(files, arg ?? "", readFile, (f, t) => writeFileSync(f, t));
      console.log(`Version ${arg} written to:\n  ${changed.join("\n  ")}`);
    } else {
      console.error("Usage: node scripts/version.mjs show | set <x.y.z> | check [vX.Y.Z]");
      process.exit(1);
    }
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
}
