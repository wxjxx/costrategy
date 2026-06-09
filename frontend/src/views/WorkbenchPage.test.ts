import { flushPromises, mount } from '@vue/test-utils';
import { computed, defineComponent, type PropType } from 'vue';
import { beforeEach, describe, expect, test, vi } from 'vitest';
import WorkbenchPage from './WorkbenchPage.vue';
import type { TaskItem } from '../features/tasks/taskModel';

const api = vi.hoisted(() => ({
  createTask: vi.fn(),
  createTaskComment: vi.fn(),
  deleteTaskAttachment: vi.fn(),
  fetchTaskDetail: vi.fn(),
  fetchTasks: vi.fn(),
  getAttachmentDownloadUrl: vi.fn(
    (taskId: string, attachmentId: string) =>
      `/api/tasks/${taskId}/attachments/${attachmentId}/download`,
  ),
  uploadTaskAttachment: vi.fn(),
  updateTask: vi.fn(),
  updateTaskStatus: vi.fn(),
}));
const elementPlus = vi.hoisted(() => ({
  ElMessage: {
    error: vi.fn(),
  },
}));

vi.mock('../features/tasks/taskApi', () => api);
vi.mock('element-plus', () => elementPlus);

describe('WorkbenchPage', () => {
  beforeEach(() => {
    api.createTask.mockReset();
    api.fetchTasks.mockReset();
    api.fetchTaskDetail.mockReset();
    api.createTaskComment.mockReset();
    api.uploadTaskAttachment.mockReset();
    api.deleteTaskAttachment.mockReset();
    api.updateTask.mockReset();
    api.updateTaskStatus.mockReset();
    elementPlus.ElMessage.error.mockReset();
    api.fetchTasks.mockResolvedValue([task()]);
    api.fetchTaskDetail.mockResolvedValue(taskDetail());
    api.createTaskComment.mockResolvedValue({
      id: 'comment-2',
      task_id: 'task-1',
      author_id: 'user-1',
      author_name: '张三',
      content: '请确认交付物',
      created_at: '2026-06-09T11:00:00Z',
    });
    api.uploadTaskAttachment.mockResolvedValue({
      id: 'attachment-1',
      task_id: 'task-1',
      file_name: '需求说明.txt',
      file_size: 12,
      mime_type: 'text/plain',
      uploader_id: 'user-1',
      uploader_name: '张三',
      created_at: '2026-06-09T12:00:00Z',
    });
    api.deleteTaskAttachment.mockResolvedValue({
      id: 'attachment-1',
      task_id: 'task-1',
      file_name: '需求说明.txt',
      file_size: 12,
      mime_type: 'text/plain',
      uploader_id: 'user-1',
      uploader_name: '张三',
      created_at: '2026-06-09T12:00:00Z',
    });
    api.updateTask.mockResolvedValue(
      task({
        project_id: 'project-2',
        project_name: '交付平台',
        title: '需求文档复核',
        assignee_id: 'user-3',
        assignee_name: '王五',
        status: 'in_progress',
        priority: 'medium',
        start_date: '2026-06-02',
        due_date: '2026-06-15',
        description_json: {
          type: 'doc',
          content: [{ type: 'paragraph', content: [{ type: 'text', text: '更新说明' }] }],
        },
      }),
    );
    api.createTask.mockResolvedValue(
      task({
        id: 'task-2',
        project_id: 'project-2',
        project_name: '交付平台',
        title: '接口联调',
        assignee_id: 'user-3',
        assignee_name: '王五',
        status: 'todo',
        priority: 'high',
        start_date: '2026-06-03',
        due_date: '2026-06-20',
        description_json: {
          type: 'doc',
          content: [{ type: 'paragraph', content: [{ type: 'text', text: '新增说明' }] }],
        },
      }),
    );
  });

  test('loads tasks into kanban, switches to list, and opens task detail', async () => {
    const wrapper = mountWorkbench();
    await flushPromises();

    expect(api.fetchTasks).toHaveBeenCalledWith(
      expect.objectContaining({
        keyword: '',
        project_id: '',
        assignee_id: '',
        status: '',
        priority: '',
      }),
    );
    expect(wrapper.get('[data-test="task-card-task-1"]').text()).toContain('需求文档确认');

    await wrapper.get('[data-test="view-tab-list"]').trigger('click');
    expect(wrapper.get('[data-test="task-row-task-1"]').text()).toContain('项目管理系统');

    await wrapper.get('[data-test="view-tab-kanban"]').trigger('click');
    await wrapper.get('[data-test="task-card-task-1"]').trigger('click');
    await flushPromises();
    expect(api.fetchTaskDetail).toHaveBeenCalledWith('task-1');
    expect(wrapper.get('[data-test="task-detail"]').text()).toContain('完成需求确认');
    expect(wrapper.get('[data-test="task-detail"]').text()).toContain('今天已确认需求');
  });

  test('submits a new comment from task detail panel', async () => {
    const wrapper = mountWorkbench();
    await flushPromises();

    await wrapper.get('[data-test="task-card-task-1"]').trigger('click');
    await flushPromises();
    await wrapper.get('[data-test="comment-input"]').setValue('  请确认交付物  ');
    await wrapper.get('[data-test="comment-form"]').trigger('submit');
    await flushPromises();

    expect(api.createTaskComment).toHaveBeenCalledWith('task-1', '请确认交付物');
    expect(wrapper.get('[data-test="task-detail"]').text()).toContain('请确认交付物');
  });

  test('uploads and deletes attachment from task detail panel', async () => {
    const wrapper = mountWorkbench();
    await flushPromises();

    await wrapper.get('[data-test="task-card-task-1"]').trigger('click');
    await flushPromises();
    const file = new File(['附件内容'], '需求说明.txt', { type: 'text/plain' });
    const input = wrapper.get('[data-test="attachment-input"]').element as HTMLInputElement;
    Object.defineProperty(input, 'files', {
      configurable: true,
      value: [file],
    });
    await wrapper.get('[data-test="attachment-input"]').trigger('change');
    await flushPromises();
    expect(api.uploadTaskAttachment).toHaveBeenCalledWith('task-1', file);
    expect(wrapper.get('[data-test="task-detail"]').text()).toContain('需求说明.txt');

    await wrapper.get('[data-test="delete-attachment-attachment-1"]').trigger('click');
    await flushPromises();
    expect(api.deleteTaskAttachment).toHaveBeenCalledWith('task-1', 'attachment-1');
    expect(wrapper.get('[data-test="task-detail"]').text()).toContain('暂无附件');
  });

  test('saves task edits from detail panel and updates workbench task state', async () => {
    const wrapper = mountWorkbench();
    await flushPromises();

    await wrapper.get('[data-test="task-card-task-1"]').trigger('click');
    await flushPromises();
    await wrapper.get('[data-test="edit-task-button"]').trigger('click');
    await wrapper.get('[data-test="edit-title"]').setValue('需求文档复核');
    await wrapper.get('[data-test="edit-project-id"]').setValue('project-2');
    await wrapper.get('[data-test="edit-assignee-id"]').setValue('user-3');
    await wrapper.get('[data-test="edit-status"]').setValue('in_progress');
    await wrapper.get('[data-test="edit-priority"]').setValue('medium');
    await wrapper.get('[data-test="edit-start-date"]').setValue('2026-06-02');
    await wrapper.get('[data-test="edit-due-date"]').setValue('2026-06-15');
    await wrapper.get('[data-test="edit-description"]').setValue('更新说明');
    await wrapper.get('[data-test="task-edit-form"]').trigger('submit');
    await flushPromises();

    expect(api.updateTask).toHaveBeenCalledWith('task-1', {
      project_id: 'project-2',
      title: '需求文档复核',
      assignee_id: 'user-3',
      status: 'in_progress',
      priority: 'medium',
      start_date: '2026-06-02',
      due_date: '2026-06-15',
      description_json: {
        type: 'doc',
        content: [
          {
            type: 'paragraph',
            content: [{ type: 'text', text: '更新说明' }],
          },
        ],
      },
    });
    expect(wrapper.get('[data-test="task-detail"]').text()).toContain('需求文档复核');

    await wrapper.get('[data-test="view-tab-list"]').trigger('click');
    expect(wrapper.get('[data-test="task-row-task-1"]').text()).toContain('需求文档复核');
  });

  test('shows task edit errors with message component', async () => {
    api.updateTask.mockRejectedValueOnce(apiError('TASK_NOT_ASSIGNEE'));
    const wrapper = mountWorkbench();
    await flushPromises();

    await wrapper.get('[data-test="task-card-task-1"]').trigger('click');
    await flushPromises();
    await wrapper.get('[data-test="edit-task-button"]').trigger('click');
    await wrapper.get('[data-test="edit-title"]').setValue('需求文档复核');
    await wrapper.get('[data-test="task-edit-form"]').trigger('submit');
    await flushPromises();

    expect(elementPlus.ElMessage.error).toHaveBeenCalledWith('只能更新自己负责的任务');
    expect(wrapper.find('[data-test="task-edit-form"]').exists()).toBe(true);
    expect(wrapper.find('[data-test="task-error"]').exists()).toBe(false);
  });

  test('creates task from workbench and updates current views', async () => {
    const wrapper = mountWorkbench();
    await flushPromises();

    await wrapper.get('[data-test="create-task-button"]').trigger('click');
    await wrapper.get('[data-test="edit-title"]').setValue('接口联调');
    await wrapper.get('[data-test="edit-project-id"]').setValue('project-2');
    await wrapper.get('[data-test="edit-assignee-id"]').setValue('user-3');
    await wrapper.get('[data-test="edit-status"]').setValue('todo');
    await wrapper.get('[data-test="edit-priority"]').setValue('high');
    await wrapper.get('[data-test="edit-start-date"]').setValue('2026-06-03');
    await wrapper.get('[data-test="edit-due-date"]').setValue('2026-06-20');
    await wrapper.get('[data-test="edit-description"]').setValue('新增说明');
    await wrapper.get('[data-test="task-edit-form"]').trigger('submit');
    await flushPromises();

    expect(api.createTask).toHaveBeenCalledWith({
      project_id: 'project-2',
      title: '接口联调',
      assignee_id: 'user-3',
      status: 'todo',
      priority: 'high',
      start_date: '2026-06-03',
      due_date: '2026-06-20',
      description_json: {
        type: 'doc',
        content: [
          {
            type: 'paragraph',
            content: [{ type: 'text', text: '新增说明' }],
          },
        ],
      },
    });
    expect(wrapper.get('[data-test="task-card-task-2"]').text()).toContain('接口联调');

    await wrapper.get('[data-test="view-tab-list"]').trigger('click');
    expect(wrapper.get('[data-test="task-row-task-2"]').text()).toContain('交付平台');
  });

  test('shows task create errors with message component and keeps form open', async () => {
    api.createTask.mockRejectedValueOnce(apiError('AUTH_FORBIDDEN'));
    const wrapper = mountWorkbench();
    await flushPromises();

    await wrapper.get('[data-test="create-task-button"]').trigger('click');
    await wrapper.get('[data-test="edit-title"]').setValue('接口联调');
    await wrapper.get('[data-test="edit-project-id"]').setValue('project-2');
    await wrapper.get('[data-test="edit-assignee-id"]').setValue('user-3');
    await wrapper.get('[data-test="task-edit-form"]').trigger('submit');
    await flushPromises();

    expect(elementPlus.ElMessage.error).toHaveBeenCalledWith('当前账号没有操作权限');
    expect(wrapper.find('[data-test="task-edit-form"]').exists()).toBe(true);
    expect(wrapper.find('[data-test="task-error"]').exists()).toBe(false);
  });

  test('submits filled filters when querying tasks', async () => {
    const wrapper = mountWorkbench();
    await flushPromises();
    api.fetchTasks.mockClear();

    await wrapper.get('[data-test="filter-toggle"]').trigger('click');
    await wrapper.get('[data-test="filter-keyword"]').setValue('  需求  ');
    await wrapper.get('[data-test="filter-status"]').setValue('todo');
    await wrapper.get('[data-test="task-filters"]').trigger('submit');
    await flushPromises();

    expect(api.fetchTasks).toHaveBeenCalledWith(
      expect.objectContaining({
        keyword: '  需求  ',
        status: 'todo',
      }),
    );
  });

  test('toggles workbench filter popover from toolbar', async () => {
    const wrapper = mountWorkbench();
    await flushPromises();

    expect(wrapper.find('[data-test="task-filters"]').exists()).toBe(false);

    await wrapper.get('[data-test="filter-toggle"]').trigger('click');
    expect(wrapper.find('[data-test="task-filters"]').exists()).toBe(true);

    await wrapper.get('[data-test="filter-close"]').trigger('click');
    expect(wrapper.find('[data-test="task-filters"]').exists()).toBe(false);
  });

  test('closes filter popover when opening task detail', async () => {
    const wrapper = mountWorkbench();
    await flushPromises();

    await wrapper.get('[data-test="filter-toggle"]').trigger('click');
    expect(wrapper.find('[data-test="task-filters"]').exists()).toBe(true);

    await wrapper.get('[data-test="task-card-task-1"]').trigger('click');
    await flushPromises();

    expect(wrapper.find('[data-test="task-filters"]').exists()).toBe(false);
    expect(wrapper.find('[data-test="task-detail"]').exists()).toBe(true);
  });

  test('shows api errors with message component instead of inline feedback', async () => {
    api.fetchTasks.mockRejectedValueOnce(apiError('TASK_NOT_ASSIGNEE'));
    const wrapper = mountWorkbench();
    await flushPromises();

    expect(elementPlus.ElMessage.error).toHaveBeenCalledWith('只能更新自己负责的任务');
    expect(wrapper.find('[data-test="task-error"]').exists()).toBe(false);
  });

  test('shows task detail load error with message component', async () => {
    api.fetchTaskDetail.mockRejectedValueOnce(apiError('RESOURCE_NOT_FOUND'));
    const wrapper = mountWorkbench();
    await flushPromises();

    await wrapper.get('[data-test="task-card-task-1"]').trigger('click');
    await flushPromises();

    expect(elementPlus.ElMessage.error).toHaveBeenCalledWith('数据不存在或已被删除');
    expect(wrapper.find('[data-test="detail-error"]').exists()).toBe(false);
  });
});

function mountWorkbench() {
  return mount(WorkbenchPage, {
    global: {
      stubs: {
        Draggable: draggableStub(),
        RichTextEditor: richTextEditorStub(),
      },
    },
  });
}

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

function richTextEditorStub() {
  return defineComponent({
    name: 'RichTextEditor',
    props: {
      modelValue: {
        type: null as unknown as PropType<unknown>,
        default: undefined,
      },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
      const plainText = computed(() => collectText(props.modelValue));

      function handleInput(event: Event) {
        const input = event.target as HTMLTextAreaElement;
        emit('update:modelValue', descriptionDoc(input.value));
      }

      return {
        plainText,
        handleInput,
      };
    },
    template: '<textarea data-test="edit-description" :value="plainText" @input="handleInput" />',
  });
}

function descriptionDoc(text: string) {
  return {
    type: 'doc',
    content: [
      {
        type: 'paragraph',
        content: text ? [{ type: 'text', text }] : [],
      },
    ],
  };
}

function collectText(node: unknown): string {
  if (!node || typeof node !== 'object') {
    return '';
  }

  const record = node as { text?: unknown; content?: unknown };
  if (typeof record.text === 'string') {
    return record.text;
  }

  if (!Array.isArray(record.content)) {
    return '';
  }

  return record.content.map(collectText).join('\n');
}

function taskDetail() {
  return {
    task: task({
      description_json: {
        type: 'doc',
        content: [
          {
            type: 'paragraph',
            content: [{ type: 'text', text: '完成需求确认' }],
          },
        ],
      },
    }),
    comments: [
      {
        id: 'comment-1',
        task_id: 'task-1',
        author_id: 'user-2',
        author_name: '李四',
        content: '今天已确认需求',
        created_at: '2026-06-09T10:00:00Z',
      },
    ],
    attachments: [],
    activity_logs: [],
  };
}

function draggableStub() {
  return {
    name: 'Draggable',
    props: ['modelValue'],
    template: `
      <div>
        <slot
          v-for="element in modelValue"
          name="item"
          :element="element"
          :key="element.id"
        />
      </div>
    `,
  };
}

function apiError(code: string) {
  return {
    isAxiosError: true,
    response: {
      data: {
        error: {
          code,
          message: 'backend fallback message',
        },
      },
    },
  };
}
