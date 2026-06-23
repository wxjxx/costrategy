export type AttachmentPreviewKind = "docx" | "excel" | "image" | "pdf" | "pptx";

const extensionPreviewKinds: Record<string, AttachmentPreviewKind> = {
  docx: "docx",
  xls: "excel",
  xlsx: "excel",
  pdf: "pdf",
  pptx: "pptx",
  bmp: "image",
  gif: "image",
  jpeg: "image",
  jpg: "image",
  png: "image",
  webp: "image",
};

const mimePreviewKinds: Record<string, AttachmentPreviewKind> = {
  "application/pdf": "pdf",
  "application/msword": "docx",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document": "docx",
  "application/vnd.ms-excel": "excel",
  "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet": "excel",
  "application/vnd.ms-powerpoint": "pptx",
  "application/vnd.openxmlformats-officedocument.presentationml.presentation": "pptx",
  "image/bmp": "image",
  "image/gif": "image",
  "image/jpeg": "image",
  "image/png": "image",
  "image/webp": "image",
};

export function attachmentPreviewKind(
  fileName: string,
  mimeType?: string,
): AttachmentPreviewKind | undefined {
  const extension = fileName.split(".").pop()?.toLowerCase();
  if (extension && extensionPreviewKinds[extension]) {
    return extensionPreviewKinds[extension];
  }

  const normalizedMimeType = mimeType?.split(";")[0]?.trim().toLowerCase();
  return normalizedMimeType ? mimePreviewKinds[normalizedMimeType] : undefined;
}
