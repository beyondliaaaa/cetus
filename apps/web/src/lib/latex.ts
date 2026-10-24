/**
 * LaTeX → Cetus expression conversion for the formula editor.
 *
 * MathLive emits LaTeX; the kernel consumes a small, strict ASCII expression
 * language (§5.2). This module is a deliberate subset converter: it accepts the
 * arithmetic that a model author can type into the formula field and rejects
 * everything else with a structured, translatable error. It never drops
 * characters silently — an unsupported construct is an error, not a guess.
 *
 * Errors carry a code (plus an optional detail such as the offending command)
 * so the UI can render localized copy while the parser stays free of i18n.
 */

// allow: SIZE_OK — the parser methods form one indivisible recursive-descent
// state machine; splitting the grammar across files would make it harder to audit.

/** Every error the converter can report; each needs an `editor.latex.<code>` message. */
export const LATEX_ERROR_CODES = [
  "empty",
  "unknown-command",
  "sum",
  "integral",
  "matrix",
  "subscript",
  "assignment",
  "missing-brace",
  "unexpected",
  "text",
  "function-argument",
  "log-base",
  "log-base-unsupported",
] as const;

export type LatexErrorCode = (typeof LATEX_ERROR_CODES)[number];

export interface LatexSuccess {
  ok: true;
  ascii: string;
}

export interface LatexFailure {
  ok: false;
  code: LatexErrorCode;
  detail: string | null;
}

export type LatexResult = LatexSuccess | LatexFailure;

class LatexParseError extends Error {
  readonly code: LatexErrorCode;
  readonly detail: string | null;

  constructor(code: LatexErrorCode, detail: string | null = null) {
    super(detail === null ? code : `${code}: ${detail}`);
    this.name = "LatexParseError";
    this.code = code;
    this.detail = detail;
  }
}

/** LaTeX commands that lower directly to a Cetus function call. */
const FUNCTIONS: Readonly<Record<string, string>> = {
  sin: "sin",
  cos: "cos",
  tan: "tan",
  arcsin: "asin",
  arccos: "acos",
  arctan: "atan",
  ln: "ln",
  exp: "exp",
  // `\lg` is the common logarithm in Chinese (and ISO 80000-2) notation.
  lg: "log10",
  // `\log` is deliberately absent: without a base it means ln in much of
  // mathematics and log10 in much of school science. The kernel has no
  // ambiguous `log` either (only `ln` / `log10`), so `\log` must name its base;
  // see `parseLog`.
};

/** Bases accepted for `\log_{b}`, mapped to the kernel function. */
const LOG_BASES: Readonly<Record<string, string>> = {
  "10": "log10",
  e: "ln",
};

/** Constructs that have no Cetus equivalent, with their user-facing category. */
const FORBIDDEN: Readonly<Record<string, LatexErrorCode>> = {
  sum: "sum",
  prod: "sum",
  coprod: "sum",
  int: "integral",
  iint: "integral",
  iiint: "integral",
  oint: "integral",
  begin: "matrix",
  end: "matrix",
  matrix: "matrix",
  pmatrix: "matrix",
  bmatrix: "matrix",
  vmatrix: "matrix",
  cases: "matrix",
};

/** Spacing and delimiter commands that carry no mathematical meaning here. */
const TRIVIA: ReadonlySet<string> = new Set([
  "left",
  "right",
  ",",
  ";",
  ":",
  "!",
  " ",
  "quad",
  "qquad",
  "enspace",
]);

const WHITESPACE: ReadonlySet<string> = new Set([" ", "\t", "\n", "\r"]);
const IDENTIFIER_TEXT = /^[A-Za-z][A-Za-z0-9_]*$/;

function isDigit(character: string): boolean {
  return character >= "0" && character <= "9";
}

function isLetter(character: string): boolean {
  return /[A-Za-z]/.test(character);
}

class Parser {
  private index = 0;

  constructor(private readonly source: string) {}

  parse(): string {
    const output = this.parseExpression(null);
    this.skipTrivia();
    if (!this.atEnd()) {
      this.fail("unexpected", this.source.slice(this.index, this.index + 12));
    }
    return output;
  }

  private atEnd(): boolean {
    return this.index >= this.source.length;
  }

  private peek(): string | null {
    return this.source[this.index] ?? null;
  }

  private advance(): string {
    const character = this.peek();
    if (character === null) {
      this.fail("unexpected", "end of formula");
    }
    this.index += 1;
    return character;
  }

  private fail(code: LatexErrorCode, detail: string | null = null): never {
    throw new LatexParseError(code, detail);
  }

  private skipWhitespace(): void {
    while (!this.atEnd()) {
      const character = this.peek();
      if (character === null || !WHITESPACE.has(character)) {
        return;
      }
      this.index += 1;
    }
  }

  /** Skips whitespace and invisible spacing/delimiter commands (`\left`, `\,`…). */
  private skipTrivia(): void {
    for (;;) {
      this.skipWhitespace();
      if (this.peek() !== "\\") {
        return;
      }
      const saved = this.index;
      const command = this.readCommand();
      if (TRIVIA.has(command.name)) {
        if (
          (command.name === "left" || command.name === "right") &&
          this.peek() === "."
        ) {
          this.index += 1;
        }
        continue;
      }
      this.index = saved;
      return;
    }
  }

  private readCommand(): { name: string; raw: string } {
    const start = this.index;
    if (this.peek() !== "\\") {
      this.fail("unexpected", "expected a command");
    }
    this.index += 1;
    let name = "";
    const first = this.peek();
    if (first !== null && isLetter(first)) {
      while (!this.atEnd()) {
        const character = this.peek();
        if (character === null || !isLetter(character)) {
          break;
        }
        name += this.advance();
      }
    } else if (first !== null) {
      name = this.advance();
    }
    if (name === "") {
      this.fail("unexpected", "\\");
    }
    return { name, raw: this.source.slice(start, this.index) };
  }

  private parseExpression(stop: string | null): string {
    let left = this.parseTerm(stop);
    for (;;) {
      this.skipTrivia();
      const character = this.peek();
      if (character !== "+" && character !== "-") {
        return left;
      }
      this.index += 1;
      const right = this.parseTerm(stop);
      left = `${left}${character}${right}`;
    }
  }

  private parseTerm(stop: string | null): string {
    let left = this.parseUnary(stop);
    for (;;) {
      this.skipTrivia();
      const character = this.peek();
      if (character === null || (stop !== null && character === stop)) {
        return left;
      }
      if (character === "*") {
        this.index += 1;
        left = `${left}*${this.parseUnary(stop)}`;
        continue;
      }
      if (character === "/") {
        this.index += 1;
        left = `${left}/${this.parseUnary(stop)}`;
        continue;
      }
      if (character === "_") {
        this.fail("subscript");
      }
      if (character === "=") {
        this.fail("assignment");
      }
      if (character === "&") {
        this.fail("matrix");
      }
      if (character === "\\") {
        const saved = this.index;
        const command = this.readCommand();
        if (command.name === "cdot" || command.name === "times") {
          left = `${left}*${this.parseUnary(stop)}`;
          continue;
        }
        if (command.name === "div") {
          left = `${left}/${this.parseUnary(stop)}`;
          continue;
        }
        this.index = saved;
        // Any other command starts an atom; implicit multiplication applies.
        left = `${left}*${this.parseUnary(stop)}`;
        continue;
      }
      if (this.startsAtom(character)) {
        left = `${left}*${this.parseUnary(stop)}`;
        continue;
      }
      return left;
    }
  }

  private startsAtom(character: string): boolean {
    return (
      isDigit(character) ||
      character === "." ||
      isLetter(character) ||
      character === "(" ||
      character === "{"
    );
  }

  private parseUnary(stop: string | null): string {
    this.skipTrivia();
    const character = this.peek();
    if (character === "+" || character === "-") {
      this.index += 1;
      return `${character}${this.parseUnary(stop)}`;
    }
    return this.parsePower(stop);
  }

  private parsePower(stop: string | null): string {
    const base = this.parseAtom(stop);
    this.skipTrivia();
    if (this.peek() !== "^") {
      return base;
    }
    this.index += 1;
    const exponent = this.parseExponent();
    return `${base}^(${exponent})`;
  }

  private parseExponent(): string {
    this.skipTrivia();
    if (this.peek() === "{") {
      return this.parseBracedExpression();
    }
    return this.parseUnary(null);
  }

  private parseAtom(stop: string | null): string {
    this.skipTrivia();
    const character = this.peek();
    if (character === null) {
      this.fail("unexpected", "end of formula");
    }
    if (stop !== null && character === stop) {
      this.fail("empty");
    }
    if (character === "(") {
      this.index += 1;
      const inner = this.parseExpression(")");
      this.expect(")");
      return `(${inner})`;
    }
    if (character === "{") {
      return `(${this.parseBracedExpression()})`;
    }
    if (character === "_") {
      this.fail("subscript");
    }
    if (character === "=") {
      this.fail("assignment");
    }
    if (character === "&") {
      this.fail("matrix");
    }
    if (character === "}") {
      this.fail("unexpected", "}");
    }
    if (character === "]") {
      this.fail("unexpected", "]");
    }
    if (character === "\\") {
      const command = this.readCommand();
      return this.commandAtom(command.name);
    }
    if (isDigit(character) || character === ".") {
      return this.readNumber();
    }
    if (isLetter(character)) {
      return this.readIdentifier();
    }
    this.fail("unexpected", character);
  }

  private commandAtom(name: string): string {
    const forbidden = FORBIDDEN[name];
    if (forbidden !== undefined) {
      this.fail(forbidden);
    }
    if (name === "_") {
      this.fail("subscript");
    }
    if (name === "&" || name === "\\") {
      this.fail("matrix");
    }
    if (name === "pi") {
      return "pi";
    }
    if (name === "frac") {
      const numerator = this.parseGroupOrAtom();
      const denominator = this.parseGroupOrAtom();
      return `((${numerator})/(${denominator}))`;
    }
    if (name === "sqrt") {
      return this.parseSqrt();
    }
    if (name === "log") {
      return this.parseLog();
    }
    const fn = FUNCTIONS[name];
    if (fn !== undefined) {
      return this.parseFunction(fn, name);
    }
    if (name === "mathrm" || name === "text") {
      return this.parseIdentifierCommand();
    }
    this.fail("unknown-command", `\\${name}`);
  }

  private parseGroupOrAtom(): string {
    this.skipTrivia();
    const character = this.peek();
    if (character === null) {
      this.fail("missing-brace");
    }
    if (character === "{") {
      return this.parseBracedExpression();
    }
    if (character === "(") {
      this.index += 1;
      const inner = this.parseExpression(")");
      this.expect(")");
      return inner;
    }
    return this.parseUnary(null);
  }

  private parseSqrt(): string {
    this.skipTrivia();
    let index: string | null = null;
    if (this.peek() === "[") {
      this.index += 1;
      index = this.parseExpression("]");
      this.expect("]");
    }
    const argument = this.parseGroupOrAtom();
    if (index === null) {
      return `sqrt(${argument})`;
    }
    return `(${argument})^(1/(${index}))`;
  }

  /**
   * `\log` must carry an explicit base: `\log_{10}` → log10, `\log_e` → ln.
   * A bare `\log` is `log-base`; any other base is `log-base-unsupported`
   * (detail = the base). Neither is guessed.
   */
  private parseLog(): string {
    this.skipTrivia();
    if (this.peek() !== "_") {
      this.fail("log-base");
    }
    this.index += 1;
    this.skipWhitespace();
    let base: string;
    if (this.peek() === "{") {
      base = this.readRawGroup().replace(/\s+/g, "");
    } else {
      const character = this.peek();
      if (character === null) {
        this.fail("log-base");
      }
      // An unbraced subscript is a single token in LaTeX: `\log_10` is base 1.
      base = this.advance();
    }
    if (base === "") {
      this.fail("log-base");
    }
    const fn = LOG_BASES[base];
    if (fn === undefined) {
      this.fail("log-base-unsupported", base);
    }
    return this.parseFunction(fn, "log");
  }

  private parseFunction(fn: string, command: string): string {
    this.skipTrivia();
    let exponent: string | null = null;
    if (this.peek() === "^") {
      this.index += 1;
      exponent = this.parseExponent();
    }
    const argument = this.parseFunctionArgument(command);
    const call = `${fn}(${argument})`;
    return exponent === null ? call : `(${call})^(${exponent})`;
  }

  private parseFunctionArgument(command: string): string {
    this.skipTrivia();
    const character = this.peek();
    if (character === null) {
      this.fail("function-argument", `\\${command}`);
    }
    if (character === "(") {
      this.index += 1;
      const inner = this.parseExpression(")");
      this.expect(")");
      return inner;
    }
    if (character === "{") {
      return this.parseBracedExpression();
    }
    if (character === "\\" || this.startsAtom(character)) {
      return this.parseUnary(null);
    }
    this.fail("function-argument", `\\${command}`);
  }

  private parseIdentifierCommand(): string {
    const raw = this.readRawGroup();
    const cleaned = raw.replace(/\\_/g, "_").trim();
    if (!IDENTIFIER_TEXT.test(cleaned)) {
      this.fail("text", cleaned.slice(0, 24));
    }
    return cleaned;
  }

  private readRawGroup(): string {
    this.skipTrivia();
    if (this.peek() !== "{") {
      this.fail("missing-brace");
    }
    this.index += 1;
    let depth = 1;
    let output = "";
    while (!this.atEnd()) {
      const character = this.advance();
      if (character === "{") {
        depth += 1;
        output += character;
        continue;
      }
      if (character === "}") {
        depth -= 1;
        if (depth === 0) {
          return output;
        }
        output += character;
        continue;
      }
      output += character;
    }
    this.fail("missing-brace");
  }

  private parseBracedExpression(): string {
    this.skipTrivia();
    if (this.peek() !== "{") {
      this.fail("missing-brace");
    }
    this.index += 1;
    const inner = this.parseExpression("}");
    this.expect("}");
    return inner;
  }

  private expect(character: string): void {
    this.skipTrivia();
    if (this.peek() !== character) {
      if (character === "}") {
        this.fail("missing-brace");
      }
      this.fail("unexpected", `expected ${character}`);
    }
    this.index += 1;
  }

  private readNumber(): string {
    let output = "";
    while (!this.atEnd()) {
      const character = this.peek();
      if (character === null || !isDigit(character)) {
        break;
      }
      output += this.advance();
    }
    if (this.peek() === ".") {
      output += this.advance();
      while (!this.atEnd()) {
        const character = this.peek();
        if (character === null || !isDigit(character)) {
          break;
        }
        output += this.advance();
      }
    }
    if (output === ".") {
      this.fail("unexpected", ".");
    }
    const exponent = this.peek();
    if (exponent === "e" || exponent === "E") {
      const saved = this.index;
      let candidate = this.advance();
      const sign = this.peek();
      if (sign === "+" || sign === "-") {
        candidate += this.advance();
      }
      let digits = "";
      while (!this.atEnd()) {
        const character = this.peek();
        if (character === null || !isDigit(character)) {
          break;
        }
        digits += this.advance();
      }
      if (digits === "") {
        this.index = saved;
      } else {
        output += `${candidate}${digits}`;
      }
    }
    return output;
  }

  private readIdentifier(): string {
    let output = "";
    while (!this.atEnd()) {
      const character = this.peek();
      if (character === null || (!isLetter(character) && !isDigit(character))) {
        break;
      }
      output += this.advance();
    }
    if (this.peek() === "_") {
      this.fail("subscript");
    }
    return output;
  }
}

/**
 * Converts a LaTeX fragment into the Cetus ASCII expression language.
 *
 * The output is fully parenthesised where precedence could be lost, e.g.
 * `\frac{a}{b}` → `((a)/(b))` and `x^{2}` → `x^(2)`.
 */
export function latexToAscii(source: string): LatexResult {
  if (source.trim() === "") {
    return { ok: false, code: "empty", detail: null };
  }
  try {
    return { ok: true, ascii: new Parser(source).parse() };
  } catch (error) {
    if (error instanceof LatexParseError) {
      return { ok: false, code: error.code, detail: error.detail };
    }
    throw error;
  }
}
