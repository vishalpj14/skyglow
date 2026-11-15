// Parameter packs bundled with the app (packs/*.json), as raw JSON text.
// The text goes to the engine unchanged, so the app and the CLI read packs
// exactly the same way. An empty string means the built-in default.

const files = import.meta.glob<string>("../../../packs/*.json", {
  eager: true,
  query: "?raw",
  import: "default",
});

export interface PackChoice {
  key: string;
  text: string;
}

export const bundledPacks: PackChoice[] = Object.entries(files)
  .map(([path, text]) => ({ key: path.split("/").pop()!.replace(/\.json$/, ""), text }))
  .sort((a, b) => (a.key === "default" ? -1 : b.key === "default" ? 1 : a.key.localeCompare(b.key)));
