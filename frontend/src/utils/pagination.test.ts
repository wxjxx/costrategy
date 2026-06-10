import { describe, expect, it } from "vitest";
import { clampPage, pageRows } from "./pagination";

describe("pagination helpers", () => {
  it("returns only rows for the current page", () => {
    expect(pageRows([1, 2, 3, 4, 5], 2, 2)).toEqual([3, 4]);
  });

  it("clamps a page to the available item count", () => {
    expect(clampPage(5, 12, 10)).toBe(2);
    expect(clampPage(0, 0, 10)).toBe(1);
  });
});
