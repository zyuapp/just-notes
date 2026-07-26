import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

type CargoPackage = {
  id: string;
  name: string;
  version: string;
  license: string | null;
  license_file: string | null;
  authors: string[];
  manifest_path: string;
  repository: string | null;
};

type CargoNode = {
  id: string;
  deps: Array<{ pkg: string; dep_kinds: Array<{ kind: string | null }> }>;
};

type Notice = { label: string; text: string };
type Component = { ecosystem: string; name: string; version: string; license: string; authors?: string[]; notices: Notice[] };

const root = resolve(import.meta.dir, "..");
const output = join(root, "src-tauri/resources/ThirdPartyNotices.txt");

function noticeText(path: string): string {
  return readFileSync(path, "utf8")
    .replace(/\r\n/g, "\n")
    .split("\n")
    .map((line) => line.trimEnd())
    .join("\n")
    .trim();
}

function licenseFiles(packageRoot: string, explicit: string | null): string[] {
  if (explicit) return [resolve(packageRoot, explicit)];
  return readdirSync(packageRoot)
    .filter((name) => /^(licen[sc]e|copying|notice)/i.test(name))
    .map((name) => join(packageRoot, name))
    .filter((path) => statSync(path).isFile())
    .sort();
}

function cargoComponents(): Component[] {
  const result = Bun.spawnSync(
    ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", "aarch64-apple-darwin", "--manifest-path", "src-tauri/Cargo.toml"],
    { cwd: root },
  );
  if (!result.success) throw new Error(result.stderr.toString());
  const metadata = JSON.parse(result.stdout.toString()) as {
    packages: CargoPackage[];
    resolve: { root: string; nodes: CargoNode[] };
  };
  const nodes = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
  const included = new Set<string>();
  const visit = (id: string) => {
    if (included.has(id)) return;
    included.add(id);
    for (const dep of nodes.get(id)?.deps ?? []) {
      if (dep.dep_kinds.some(({ kind }) => kind === null)) visit(dep.pkg);
    }
  };
  visit(metadata.resolve.root);
  included.delete(metadata.resolve.root);

  const packages = metadata.packages.filter(({ id }) => included.has(id));
  const repositoryLicenses = new Map<string, string[]>();
  for (const pkg of packages) {
    const paths = licenseFiles(dirname(pkg.manifest_path), pkg.license_file);
    if (paths.length > 0 && pkg.repository) {
      repositoryLicenses.set(`${pkg.repository}|${pkg.license}`, paths);
    }
  }
  return packages.map((pkg) => {
      const packageRoot = dirname(pkg.manifest_path);
      const paths = licenseFiles(packageRoot, pkg.license_file).length > 0
        ? licenseFiles(packageRoot, pkg.license_file)
        : repositoryLicenses.get(`${pkg.repository}|${pkg.license}`) ?? [];
      const resolvedPaths = paths.length > 0 ? paths : spdxLicenseFiles(pkg.license);
      if (resolvedPaths.length === 0) {
        throw new Error(`No authoritative license text found for Rust package ${pkg.name} ${pkg.version} (${pkg.license})`);
      }
      const notices = resolvedPaths.map((path) => ({
        label: path.split("/").pop()!,
        text: noticeText(path),
      }));
      return {
        ecosystem: "Rust",
        name: pkg.name,
        version: pkg.version,
        license: pkg.license ?? "See included license text",
        authors: pkg.authors,
        notices,
      };
    });
}

function spdxLicenseFiles(expression: string | null): string[] {
  if (!expression) return [];
  const identifiers = expression.match(/MIT|Apache-2\.0|MPL-2\.0/g) ?? [];
  return [...new Set(identifiers)].map((identifier) =>
    join(root, "src-tauri/resources/licenses/spdx", `${identifier}.txt`)
  );
}

function nativeComponents(): Component[] {
  const manifestPath = join(root, "src-tauri/resources/licenses/native/manifest.json");
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as {
    archive: { name: string; sha256: string; source: string; githubAssetDigest: string };
    components: Array<{ name: string; version: string; license: string; libraries: string[]; files: string[] }>;
    excluded: Array<{ libraries: string[] }>;
  };
  validateNativeManifest(manifest);
  return manifest.components.map((component) => ({
    ecosystem: "Native",
    name: component.name,
    version: component.version,
    license: component.license,
    notices: component.files.map((file) => ({
      label: file,
      text: noticeText(resolve(dirname(manifestPath), file)),
    })),
  }));
}

function validateNativeManifest(manifest: {
  archive: { name: string; sha256: string; source: string; githubAssetDigest: string };
  components: Array<{ libraries: string[] }>;
  excluded: Array<{ libraries: string[] }>;
}) {
  const buildScript = readFileSync(join(root, "src-tauri/vendor/sherpa-onnx-sys/build.rs"), "utf8");
  if (!buildScript.includes(manifest.archive.name) || !buildScript.includes(manifest.archive.sha256)) {
    throw new Error("Native notice manifest archive does not match the verified sherpa-onnx build input");
  }
  if (
    manifest.archive.githubAssetDigest !== `sha256:${manifest.archive.sha256}` ||
    !manifest.archive.source.startsWith("https://github.com/k2-fsa/sherpa-onnx/releases/download/") ||
    !manifest.archive.source.endsWith(`/${manifest.archive.name}`)
  ) {
    throw new Error("Native notice manifest does not retain the verified official release provenance");
  }
  const staticList = buildScript.match(/const SHERPA_ONNX_STATIC_LIBS[^=]*= &\[(.*?)\];/s)?.[1];
  if (!staticList) throw new Error("Could not read sherpa-onnx static library inventory");
  const configured = new Set([...staticList.matchAll(/"([^"]+)"/g)].map((match) => match[1]));
  const inventoried = new Set(manifest.components.flatMap(({ libraries }) => libraries));
  const excluded = new Set(manifest.excluded.flatMap(({ libraries }) => libraries));
  const missing = [...configured].filter((library) => !inventoried.has(library));
  const unknown = [...inventoried].filter((library) => !configured.has(library));
  const forbidden = [...configured].filter((library) => excluded.has(library));
  if (missing.length || unknown.length || forbidden.length) {
    throw new Error(`Native notice inventory drifted from static link inputs; missing=${missing.join(",")}; unknown=${unknown.join(",")}; excluded=${forbidden.join(",")}`);
  }
}


function javascriptComponents(): Component[] {
  const packageJson = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  const pending = Object.keys(packageJson.dependencies ?? {});
  const included = new Set<string>();
  const components: Component[] = [];
  while (pending.length > 0) {
    const name = pending.pop()!;
    if (included.has(name)) continue;
    included.add(name);
    const packageRoot = join(root, "node_modules", name);
    const manifest = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8"));
    pending.push(...Object.keys(manifest.dependencies ?? {}));
    const paths = licenseFiles(packageRoot, manifest.licenseFile ?? null);
    if (paths.length === 0) throw new Error(`No license file found for JavaScript package ${name} ${manifest.version}`);
    components.push({
      ecosystem: "JavaScript",
      name,
      version: manifest.version,
      license: typeof manifest.license === "string" ? manifest.license : "See included license text",
      notices: paths.map((path) => ({ label: path.split("/").pop()!, text: noticeText(path) })),
    });
  }
  return components;
}

function generate() {
  const components = [...javascriptComponents(), ...cargoComponents(), ...nativeComponents()]
    .sort((left, right) => `${left.ecosystem}:${left.name}:${left.version}`.localeCompare(`${right.ecosystem}:${right.name}:${right.version}`));

  const texts = new Map<string, { id: string; text: string }>();
  const componentLines = components.map((component) => {
    const ids = component.notices.map(({ text }) => {
      const digest = createHash("sha256").update(text).digest("hex");
      if (!texts.has(digest)) texts.set(digest, { id: `License-${texts.size + 1}`, text });
      return texts.get(digest)!.id;
    });
    const authors = component.authors?.length ? ` — authors: ${component.authors.join(", ")}` : "";
    return `${component.ecosystem}: ${component.name} ${component.version} — ${component.license}${authors} — ${ids.join(", ")}`;
  });
  const catalog = [...texts.values()].map(({ id, text }) => `\n================================================================================\n${id}\n================================================================================\n${text}\n`).join("");
  writeFileSync(output, `Just Notes Third-Party Software Notices\n\nGenerated from the exact Cargo and JavaScript dependency graphs plus the bundled native runtime.\n\n${componentLines.join("\n")}\n${catalog}`);
}

generate();
