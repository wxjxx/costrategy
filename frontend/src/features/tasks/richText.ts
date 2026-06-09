import DOMPurify from "dompurify";

export function renderDescriptionHtml(value: Record<string, unknown>): string {
  if (typeof value.html === "string") {
    return DOMPurify.sanitize(value.html);
  }
  return DOMPurify.sanitize(`
    <h3>任务目标</h3>
    <p>按设计稿完成页面还原，联调后端任务、项目和用户接口。</p>
    <h3>具体内容</h3>
    <ol>
      <li>保持工作台筛选条件在看板、甘特图和列表间共享。</li>
      <li>任务详情展示描述、附件、评论和操作记录。</li>
      <li>员工只能更新自己负责的任务状态。</li>
    </ol>
  `);
}
