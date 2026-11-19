import { test } from "node:test";
import assert from "node:assert/strict";
import { isVersion, loadConfig, problems, readVersions, setVersions } from "./version.mjs";

const FILES = [
  { file: "package.json", kind: "json" },
  { file: "Cargo.toml", kind: "toml", section: "package" },
  { file: "pyproject.toml", kind: "toml", section: "project" },
  { file: "app/__init__.py", kind: "regex", pattern: '(__version__ = ")([^"]+)(")' },
];

const disk = () => ({
  "package.json": '{\r\n  "name": "x",\r\n  "version": "1.0.0",\r\n  "dependencies": { "y": { "version": "9.9.9" } }\r\n}\r\n',
  "Cargo.toml": '[package]\nname = "x"\nversion = "1.0.0"\n\n[dependencies]\ny = { version = "9.9.9" }\n',
  "pyproject.toml": '[build-system]\nrequires = ["a"]\n\n[project]\nname = "x"\nversion = "1.0.0"\n',
  "app/__init__.py": '__version__ = "1.0.0"\n',
});
const reader = (d) => (f) => (f in d ? d[f] : null);

test("versions are recognised", () => {
  assert.ok(isVersion("1.2.3") && isVersion("1.2.3-beta.1"));
  assert.ok(!isVersion("1.2") && !isVersion("v1.2.3") && !isVersion(""));
});

test("each kind finds its own version and not another one", () => {
  assert.deepEqual(readVersions(FILES, reader(disk())), {
    "package.json": "1.0.0",
    "Cargo.toml": "1.0.0",
    "pyproject.toml": "1.0.0",
    "app/__init__.py": "1.0.0",
  });
});

test("set changes only the project's own version and keeps every other byte", () => {
  const d = disk();
  const before = JSON.parse(JSON.stringify(d));
  const changed = setVersions(FILES, "1.1.0", reader(d), (f, t) => (d[f] = t));
  assert.equal(changed.length, 4);
  assert.equal(d["package.json"], before["package.json"].replace('"version": "1.0.0"', '"version": "1.1.0"')); // CRLF kept, nested version untouched
  assert.equal(d["Cargo.toml"], before["Cargo.toml"].replace('version = "1.0.0"', 'version = "1.1.0"'));
  assert.equal(d["pyproject.toml"], before["pyproject.toml"].replace('version = "1.0.0"', 'version = "1.1.0"'));
  assert.equal(d["app/__init__.py"], '__version__ = "1.1.0"\n');
  assert.deepEqual(problems(readVersions(FILES, reader(d)), "v1.1.0"), []);
});

test("set refuses a bad version, a missing file and a file without a version", () => {
  const d = disk();
  assert.throws(() => setVersions(FILES, "1.1", reader(d), () => {}), /not a version/);
  assert.throws(() => setVersions(FILES, "1.1.0", reader({}), () => {}), /no such file/);
  assert.throws(() => setVersions([{ file: "a.json", kind: "json" }], "1.1.0", reader({ "a.json": "{}" }), () => {}), /no version found/);
  assert.throws(() => setVersions([{ file: "a", kind: "nope" }], "1.1.0", reader({ a: "" }), () => {}), /unknown kind/);
});

test("check: a tag must match every file, and the files must agree with each other", () => {
  const d = disk();
  assert.deepEqual(problems(readVersions(FILES, reader(d)), "v1.0.0"), []);
  assert.deepEqual(problems(readVersions(FILES, reader(d))), []);
  d["Cargo.toml"] = d["Cargo.toml"].replace('version = "1.0.0"', 'version = "1.0.1"');
  assert.deepEqual(problems(readVersions(FILES, reader(d)), "v1.0.0"), ["Cargo.toml says 1.0.1, but the tag is v1.0.0."]);
  assert.deepEqual(problems(readVersions(FILES, reader(d))), ["Cargo.toml says 1.0.1, but others say 1.0.0."]);
  assert.deepEqual(problems(readVersions(FILES, reader(d)), "1.0.0"), ['The tag "1.0.0" is not like v1.2.0.']);
});

test("check says so when nothing is configured or a version is missing", () => {
  assert.deepEqual(problems({}), ["No version files are configured: list them in version-files.json."]);
  assert.deepEqual(problems({ "a.json": null }, "v1.0.0"), ["a.json: no version found."]);
});

test("the config must have a files list", () => {
  assert.deepEqual(loadConfig(() => '{"files": []}'), []);
  assert.throws(() => loadConfig(() => "{}"), /files/);
  assert.throws(() => loadConfig(() => null), /missing/);
});
