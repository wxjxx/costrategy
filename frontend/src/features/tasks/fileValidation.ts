export function isSvgFile(file: File): boolean {
  const mimeType = file.type.toLowerCase().split(";")[0]?.trim();
  return mimeType === "image/svg+xml" || file.name.toLowerCase().endsWith(".svg");
}
