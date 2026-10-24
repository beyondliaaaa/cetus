import { describe, expect, it } from "vitest";
import { IMPLEMENTED_VIEWS, pickView } from "./registry";

describe("view registry", () => {
  it("renders the series view declared by SD models", () => {
    expect(pickView(["series", "phase"])).toBe("series");
    expect(IMPLEMENTED_VIEWS).toContain("series");
  });

  it("skips declared but unimplemented views", () => {
    expect(pickView(["phase", "series"])).toBe("series");
    expect(pickView(["phase"])).toBeNull();
    expect(pickView([])).toBeNull();
  });
});
