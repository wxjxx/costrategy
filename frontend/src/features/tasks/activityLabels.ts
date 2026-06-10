const ACTION_LABELS: Record<string, string> = {
  task_created: "创建任务",
  comment_created: "新增评论",
  status_changed: "更新状态",
  schedule_changed: "更新任务",
  task_archived: "归档任务",
  attachment_uploaded: "上传附件",
  attachment_deleted: "删除附件",
};

export function activityActionLabel(action: string): string {
  return ACTION_LABELS[action] ?? action;
}
