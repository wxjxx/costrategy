import { describe, expect, it } from "vitest";
import { renderDescriptionHtml } from "./richText";

describe("renderDescriptionHtml", () => {
  it("does not render sample content when the API description is empty", () => {
    expect(renderDescriptionHtml({ type: "doc", content: [] })).toBe("");
    expect(renderDescriptionHtml({})).toBe("");
  });

  it("sanitizes API-provided html", () => {
    expect(renderDescriptionHtml({ html: "<p>接口内容</p><script>alert(1)</script>" })).toBe(
      "<p>接口内容</p>",
    );
  });

  it("renders API-provided Tiptap JSON content", () => {
    expect(
      renderDescriptionHtml({
        type: "doc",
        content: [
          {
            type: "paragraph",
            content: [{ type: "text", text: "测试任务全流程" }],
          },
        ],
      }),
    ).toBe("<p>测试任务全流程</p>");
  });

  it("renders uploaded rich text images from backend urls", () => {
    expect(
      renderDescriptionHtml({
        type: "doc",
        content: [
          {
            type: "image",
            attrs: { src: "/api/rich-text/images/image-1.png" },
          },
        ],
      }),
    ).toBe('<img src="/api/rich-text/images/image-1.png">');
  });
});
