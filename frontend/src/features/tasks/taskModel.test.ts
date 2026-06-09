import { describe, expect, test } from 'vitest';
import {
  countActiveTaskFilters,
  extractTaskDescriptionText,
  groupTasksByStatus,
  taskActivityLabel,
  taskPriorityLabel,
  taskStatusLabel,
  type TaskItem,
} from './taskModel';

describe('taskModel', () => {
  test('groups tasks into first version status columns', () => {
    const grouped = groupTasksByStatus([
      task('task-1', 'todo'),
      task('task-2', 'done'),
      task('task-3', 'in_progress'),
    ]);

    expect(grouped.todo.map((item) => item.id)).toEqual(['task-1']);
    expect(grouped.in_progress.map((item) => item.id)).toEqual(['task-3']);
    expect(grouped.done.map((item) => item.id)).toEqual(['task-2']);
  });

  test('counts only filled filters', () => {
    expect(
      countActiveTaskFilters({
        keyword: '  需求  ',
        project_id: '',
        assignee_id: undefined,
        status: 'todo',
        priority: '',
      }),
    ).toBe(2);
  });

  test('returns stable chinese labels for status and priority', () => {
    expect(taskStatusLabel('todo')).toBe('待处理');
    expect(taskStatusLabel('in_progress')).toBe('进行中');
    expect(taskStatusLabel('done')).toBe('已完成');
    expect(taskPriorityLabel('high')).toBe('高');
  });

  test('extracts readable text from tiptap style rich text description', () => {
    expect(
      extractTaskDescriptionText({
        type: 'doc',
        content: [
          {
            type: 'paragraph',
            content: [{ type: 'text', text: '完成需求确认' }],
          },
          {
            type: 'paragraph',
            content: [{ type: 'text', text: '同步排期风险' }],
          },
        ],
      }),
    ).toBe('完成需求确认\n同步排期风险');
  });

  test('returns stable chinese activity labels', () => {
    expect(taskActivityLabel('task_created')).toBe('创建任务');
    expect(taskActivityLabel('comment_created')).toBe('新增评论');
    expect(taskActivityLabel('unknown_action')).toBe('unknown_action');
  });
});

function task(id: string, status: TaskItem['status']): TaskItem {
  return {
    id,
    project_id: 'project-1',
    project_name: '项目管理系统',
    title: `任务 ${id}`,
    assignee_id: 'user-1',
    assignee_name: '张三',
    status,
    priority: 'medium',
    start_date: '2026-06-01',
    due_date: '2026-06-10',
    description_json: { type: 'doc', content: [] },
    creator_id: 'creator-1',
    archived: false,
    is_overdue: false,
    display_status: status,
  };
}
