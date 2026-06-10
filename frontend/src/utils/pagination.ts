export function clampPage(page: number, total: number, pageSize: number): number {
  const maxPage = Math.max(1, Math.ceil(total / pageSize));
  return Math.min(Math.max(1, page), maxPage);
}

export function pageRows<T>(rows: T[], page: number, pageSize: number): T[] {
  const normalizedPage = clampPage(page, rows.length, pageSize);
  const start = (normalizedPage - 1) * pageSize;
  return rows.slice(start, start + pageSize);
}
