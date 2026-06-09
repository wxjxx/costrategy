import { beforeEach, describe, expect, test, vi } from 'vitest';
import type { TaskItem } from './taskModel';

const httpMock = vi.hoisted(() => ({
  http: {
    defaults: {
      baseURL: '/api',
    },
    post: vi.fn(),
    put: vi.fn(),
  },
}));

vi.mock('../../api/http', () => httpMock);

import { buildTaskQueryParams, createTask, getAttachmentDownloadUrl, updateTask } from './taskApi';

beforeEach(() => {
  httpMock.http.post.mockReset();
  httpMock.http.put.mockReset();
});

describe('buildTaskQueryParams', () => {
  test('serializes task filters for backend list endpoint and skips empty values', () => {
    const params = buildTaskQueryParams({
      keyword: '  需求  ',
      project_id: 'project-1',
      assignee_id: '',
      status: 'todo',
      priority: 'high',
      date_from: undefined,
      date_to: '2026-06-30',
    });

    expect(Array.from(params.entries())).toEqual([
      ['keyword', '需求'],
      ['project_id', 'project-1'],
      ['status', 'todo'],
      ['priority', 'high'],
      ['date_to', '2026-06-30'],
    ]);
  });
});

describe('getAttachmentDownloadUrl', () => {
  test('builds stable backend download path for attachment links', () => {
    expect(getAttachmentDownloadUrl('task-1', 'attachment-1')).toBe(
      '/api/tasks/task-1/attachments/attachment-1/download',
    );
  });
});

describe('updateTask', () => {
  test('sends task edit payload to backend and returns updated task', async () => {
    const updated = task({ title: '需求文档复核', priority: 'medium' });
    const payload = {
      project_id: 'project-1',
      title: '需求文档复核',
      assignee_id: 'user-1',
      status: 'in_progress' as const,
      priority: 'medium' as const,
      start_date: '2026-06-02',
      due_date: '2026-06-15',
      description_json: {
        type: 'doc',
        content: [{ type: 'paragraph', content: [{ type: 'text', text: '更新说明' }] }],
      },
    };
    httpMock.http.put.mockResolvedValue({ data: updated });

    await expect(updateTask('task-1', payload)).resolves.toEqual(updated);

    expect(httpMock.http.put).toHaveBeenCalledWith('/tasks/task-1', payload);
  });
});

describe('createTask', () => {
  test('sends task create payload to backend and returns created task', async () => {
    const created = task({ id: 'task-2', title: '接口联调' });
    const payload = {
      project_id: 'project-1',
      title: '接口联调',
      assignee_id: 'user-1',
      status: 'todo' as const,
      priority: 'high' as const,
      start_date: '2026-06-03',
      due_date: '2026-06-20',
      description_json: {
        type: 'doc',
        content: [{ type: 'paragraph', content: [{ type: 'text', text: '新增说明' }] }],
      },
    };
    httpMock.http.post.mockResolvedValue({ data: created });

    await expect(createTask(payload)).resolves.toEqual(created);

    expect(httpMock.http.post).toHaveBeenCalledWith('/tasks', payload);
  });
});

function task(overrides: Partial<TaskItem> = {}): TaskItem {
  return {
    id: 'task-1',
    project_id: 'project-1',
    project_name: '项目管理系统',
    title: '需求文档确认',
    assignee_id: 'user-1',
    assignee_name: '张三',
    status: 'todo',
    priority: 'high',
    start_date: '2026-06-01',
    due_date: '2026-06-10',
    description_json: { type: 'doc', content: [] },
    creator_id: 'creator-1',
    archived: false,
    is_overdue: false,
    display_status: 'todo',
    ...overrides,
  };
}
