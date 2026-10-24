/**
 * Minimal YAML field reader for values the compiled metadata omits.
 *
 * `ModelMeta` exposes equations for flows and auxiliaries but not observables,
 * and it exposes a stock's initial value only numerically. The editor keeps the
 * authored source for those fields so opening (and applying) an editor never
 * rewrites `0.5 * capacity` into a frozen number. This is deliberately narrow:
 * it finds a top-level section, then an entry inside it, and returns one scalar.
 * It is not a general YAML parser and returns `null` instead of guessing.
 */

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function scalarValue(raw: string): string | null {
  const withoutComment = raw.replace(/\s+#.*$/, "").trim();
  if (withoutComment === "") {
    return null;
  }
  if (
    (withoutComment.startsWith('"') && withoutComment.endsWith('"')) ||
    (withoutComment.startsWith("'") && withoutComment.endsWith("'"))
  ) {
    const inner = withoutComment.slice(1, -1).trim();
    return inner === "" ? null : inner;
  }
  return withoutComment;
}

function fieldIn(text: string, field: string): string | null {
  const pattern = new RegExp(
    `\\b${escapeRegExp(field)}:\\s*(?:"([^"]*)"|'([^']*)'|([^,\\n#}]+))`,
  );
  const match = pattern.exec(text);
  if (match === null) {
    return null;
  }
  return scalarValue(match[1] ?? match[2] ?? match[3] ?? "");
}

/**
 * Reads one scalar field from `section.name.field` in the authored YAML.
 *
 * Handles both inline maps (`charge_pct: { eq: "..." }`) and block style.
 */
export function readYamlScalar(
  yaml: string,
  section: string,
  name: string,
  field: string,
): string | null {
  const lines = yaml.split(/\r?\n/);
  const heading = new RegExp(`^${escapeRegExp(section)}:\\s*(?:#.*)?$`);
  const start = lines.findIndex((line) => heading.test(line));
  if (start === -1) {
    return null;
  }

  for (let index = start + 1; index < lines.length; index += 1) {
    const line = lines[index] ?? "";
    if (/^\S/.test(line)) {
      return null;
    }
    const entry = /^(\s+)([A-Za-z_][\w-]*):\s*(.*)$/.exec(line);
    if (entry === null || entry[2] !== name) {
      continue;
    }
    const inline = fieldIn(entry[3] ?? "", field);
    if (inline !== null) {
      return inline;
    }
    const entryIndent = entry[1]?.length ?? 0;
    for (let inner = index + 1; inner < lines.length; inner += 1) {
      const nested = lines[inner] ?? "";
      const nestedIndent = /^\s*/.exec(nested)?.[0].length ?? 0;
      if (nested.trim() !== "" && nestedIndent <= entryIndent) {
        return null;
      }
      const block = new RegExp(`^\\s+${escapeRegExp(field)}:\\s*(.*)$`).exec(
        nested,
      );
      if (block !== null) {
        return scalarValue(block[1] ?? "");
      }
    }
    return null;
  }

  return null;
}

export function readYamlEquation(
  yaml: string,
  section: string,
  name: string,
): string | null {
  return readYamlScalar(yaml, section, name, "eq");
}
