import { realpathSync } from "node:fs";
import { join, resolve } from "node:path";

type CargoPackage = {
  id: string;
  manifest_path: string;
  name: string;
  source: string | null;
};

const root = resolve(import.meta.dir, "..");
const result = Bun.spawnSync(
  [
    "cargo",
    "metadata",
    "--format-version",
    "1",
    "--locked",
    "--filter-platform",
    "aarch64-apple-darwin",
    "--manifest-path",
    "src-tauri/Cargo.toml",
  ],
  { cwd: root },
);

if (!result.success) {
  throw new Error(result.stderr.toString());
}

const metadata = JSON.parse(result.stdout.toString()) as {
  packages: CargoPackage[];
  resolve: { nodes: Array<{ id: string }> };
};
const resolvedIds = new Set(metadata.resolve.nodes.map(({ id }) => id));
const sherpaSys = metadata.packages.filter(
  ({ id, name }) => name === "sherpa-onnx-sys" && resolvedIds.has(id),
);

if (sherpaSys.length !== 1) {
  throw new Error(
    `Expected exactly one resolved sherpa-onnx-sys package, found ${sherpaSys.length}`,
  );
}

const resolved = sherpaSys[0];
const expectedManifest = realpathSync(
  join(root, "src-tauri/vendor/sherpa-onnx-sys/Cargo.toml"),
);
const resolvedManifest = realpathSync(resolved.manifest_path);

if (resolved.source !== null || resolvedManifest !== expectedManifest) {
  throw new Error(
    `sherpa-onnx-sys resolved outside the vendored path: source=${resolved.source ?? "path"}, manifest=${resolvedManifest}`,
  );
}

console.log(`Verified vendored sherpa-onnx-sys at ${resolvedManifest}`);
