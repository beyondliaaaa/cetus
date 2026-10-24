import { describe, expect, it } from "vitest";
import { latexToAscii, type LatexFailure } from "./latex";

function ascii(source: string): string {
  const result = latexToAscii(source);
  if (!result.ok) {
    throw new Error(`expected success for ${source}, got ${result.code}`);
  }
  return result.ascii;
}

function failure(source: string): LatexFailure {
  const result = latexToAscii(source);
  if (result.ok) {
    throw new Error(`expected failure for ${source}, got ${result.ascii}`);
  }
  return result;
}

describe("LaTeX conversion", () => {
  it("converts fractions and square roots", () => {
    expect(ascii("\\frac{a}{b}")).toBe("((a)/(b))");
    expect(ascii("\\frac{2\\pi r}{t}")).toBe("((2*pi*r)/(t))");
    expect(ascii("\\sqrt{x}")).toBe("sqrt(x)");
    expect(ascii("\\sqrt{x^2+1}")).toBe("sqrt(x^(2)+1)");
  });

  it("converts indexed roots and powers", () => {
    expect(ascii("\\sqrt[n]{x}")).toBe("(x)^(1/(n))");
    expect(ascii("x^{2}")).toBe("x^(2)");
    expect(ascii("e^{-t}")).toBe("e^(-t)");
    expect(ascii("x^{a+b}")).toBe("x^(a+b)");
  });

  it("maps arithmetic operators and constants", () => {
    expect(ascii("a \\cdot b \\times c \\div d")).toBe("a*b*c/d");
    expect(ascii("2\\pi r")).toBe("2*pi*r");
    expect(ascii("\\pi")).toBe("pi");
  });

  it("inserts implicit multiplication without breaking calls", () => {
    expect(ascii("a(b+c)")).toBe("a*(b+c)");
    expect(ascii("(a)(b)")).toBe("(a)*(b)");
    expect(ascii("{a+b}c")).toBe("(a+b)*c");
    expect(ascii("2\\sin(x)")).toBe("2*sin(x)");
  });

  it("strips invisible delimiters and spacing", () => {
    expect(ascii("\\left(\\frac{a}{b}\\right)")).toBe("(((a)/(b)))");
    expect(ascii("a\\,b\\;c\\!d")).toBe("a*b*c*d");
  });

  it("maps function names", () => {
    expect(ascii("\\sin(x)+\\cos(x)+\\tan(x)")).toBe("sin(x)+cos(x)+tan(x)");
    expect(ascii("\\arcsin(x)+\\arccos(x)+\\arctan(x)")).toBe(
      "asin(x)+acos(x)+atan(x)",
    );
    expect(ascii("\\ln(x)+\\exp(x)+\\log(x)")).toBe("ln(x)+exp(x)+log10(x)");
    expect(ascii("\\sin^2(x)")).toBe("(sin(x))^(2)");
  });

  it("unwraps roman and text identifiers", () => {
    expect(ascii("\\mathrm{pv\\_power}+\\text{rate}")).toBe("pv_power+rate");
    expect(ascii("2.5e-3 x")).toBe("2.5e-3*x");
  });

  it("rejects non-arithmetic LaTeX with a readable code", () => {
    const sum = failure("\\sum_{i=1}^{n} x");
    expect(sum.code).toBe("sum");

    const integral = failure("\\int_0^1 x");
    expect(integral.code).toBe("integral");

    const matrix = failure("\\begin{matrix}a&b\\end{matrix}");
    expect(matrix.code).toBe("matrix");

    const subscript = failure("x_1");
    expect(subscript.code).toBe("subscript");

    const assignment = failure("y = 2");
    expect(assignment.code).toBe("assignment");

    const unknown = failure("\\foo(x)");
    expect(unknown.code).toBe("unknown-command");
    expect(unknown.detail).toBe("\\foo");
  });

  it("reports malformed input instead of dropping characters", () => {
    expect(failure("").code).toBe("empty");
    expect(failure("{}").code).toBe("empty");
    expect(failure("\\frac{a}").code).toBe("missing-brace");
    expect(failure("\\frac{a}{b").code).toBe("missing-brace");
    expect(failure("\\sin").code).toBe("function-argument");
    expect(failure("\\mathrm{pv power}").code).toBe("text");
    expect(failure("x!").code).toBe("unexpected");
    expect(failure("1 +").code).toBe("unexpected");
  });
});
