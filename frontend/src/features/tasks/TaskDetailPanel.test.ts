import { mount } from '@vue/test-utils';
import { computed, defineComponent, type PropType } from 'vue';
import { describe, expect, test } from 'vitest';
import TaskDetailPanel from './TaskDetailPanel.vue';
import type { TaskDetail } from './taskModel';

describe('TaskDetailPanel', () => {
  test('renders task rich text description, comments, attachments and activity logs', () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        detail: taskDetail({
          attachments: [
            {
              id: 'attachment-1',
              task_id: 'task-1',
              file_name: '需求说明.txt',
              file_size: 12,
              mime_type: 'text/plain',
              uploader_id: 'user-1',
              uploader_name: '张三',
              created_at: '2026-06-09T10:02:00Z',
            },
          ],
        }),
      },
    });

    expect(wrapper.text()).toContain('需求文档确认');
    expect(wrapper.text()).toContain('完成需求确认');
    expect(wrapper.text()).toContain('今天已确认需求');
    expect(wrapper.text()).toContain('需求说明.txt');
    expect(wrapper.text()).toContain('新增评论');
  });

  test('emits trimmed comment content and ignores blank submit', async () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        detail: taskDetail(),
      },
    });

    await wrapper.get('[data-test="comment-input"]').setValue('   ');
    await wrapper.get('[data-test="comment-form"]').trigger('submit');
    expect(wrapper.emitted('submit-comment')).toBeUndefined();

    await wrapper.get('[data-test="comment-input"]').setValue('  请确认交付物  ');
    await wrapper.get('[data-test="comment-form"]').trigger('submit');
    expect(wrapper.emitted('submit-comment')).toEqual([['请确认交付物']]);
  });

  test('emits selected file for upload and attachment id for delete', async () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        detail: taskDetail({
          attachments: [
            {
              id: 'attachment-1',
              task_id: 'task-1',
              file_name: '需求说明.txt',
              file_size: 12,
              mime_type: 'text/plain',
              uploader_id: 'user-1',
              uploader_name: '张三',
              created_at: '2026-06-09T10:02:00Z',
            },
          ],
        }),
      },
    });
    const file = new File(['附件内容'], '补充说明.txt', { type: 'text/plain' });
    const input = wrapper.get('[data-test="attachment-input"]').element as HTMLInputElement;
    Object.defineProperty(input, 'files', {
      configurable: true,
      value: [file],
    });

    await wrapper.get('[data-test="attachment-input"]').trigger('change');
    await wrapper.get('[data-test="delete-attachment-attachment-1"]').trigger('click');

    expect(wrapper.emitted('upload-attachment')).toEqual([[file]]);
    expect(wrapper.emitted('delete-attachment')).toEqual([['attachment-1']]);
  });

  test('emits task edit payload with rich text description json', async () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        detail: taskDetail(),
      },
      global: {
        stubs: {
          RichTextEditor: richTextEditorStub(),
        },
      },
    });

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

    expect(wrapper.emitted('save-task')).toEqual([
      [
        {
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
        },
      ],
    ]);
  });
});

function taskDetail(overrides: Partial<TaskDetail> = {}): TaskDetail {
  return {
    task: {
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
      description_json: {
        type: 'doc',
        content: [
          {
            type: 'paragraph',
            content: [{ type: 'text', text: '完成需求确认' }],
          },
        ],
      },
      creator_id: 'creator-1',
      archived: false,
      is_overdue: false,
      display_status: 'todo',
    },
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
    activity_logs: [
      {
        id: 'log-1',
        task_id: 'task-1',
        actor_id: 'user-2',
        actor_name: '李四',
        action: 'comment_created',
        created_at: '2026-06-09T10:01:00Z',
      },
    ],
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
