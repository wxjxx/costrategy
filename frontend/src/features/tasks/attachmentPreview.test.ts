import { describe, expect, it } from "vitest";
import { attachmentPreviewKind } from "./attachmentPreview";

describe("attachmentPreviewKind", () => {
  it.each([
    ["需求说明.DOCX", "docx"],
    ["排期.xlsx", "excel"],
    ["预算.XLS", "excel"],
    ["方案.pdf", "pdf"],
    ["汇报.PPTX", "pptx"],
  ] as const)("detects supported office preview files by extension", (fileName, kind) => {
    expect(attachmentPreviewKind(fileName)).toBe(kind);
  });

  it("falls back to known mime types when the filename has no extension", () => {
    expect(
      attachmentPreviewKind(
        "download",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
      ),
    ).toBe("pptx");
  });

  it("does not preview unsupported files", () => {
    expect(attachmentPreviewKind("notes.txt", "text/plain")).toBeUndefined();
  });
});
