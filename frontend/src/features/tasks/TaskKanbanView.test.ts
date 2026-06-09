import { mount } from '@vue/test-utils';
import { describe, expect, test } from 'vitest';
import TaskKanbanView from './TaskKanbanView.vue';
import type { TaskItem } from './taskModel';

describe('TaskKanbanView', () => {
  test('renders cards with title, project, assignee, due date, priority and overdue badge', () => {
    const wrapper = mount(TaskKanbanView, {
      props: {
        tasks: [task({ id: 'task-1', is_overdue: true, priority: 'high' })],
      },
      global: {
        stubs: {
          Draggable: draggableStub(),
        },
      },
    });

    const card = wrapper.get('[data-test="task-card-task-1"]');
    expect(card.text()).toContain('需求文档确认');
    expect(card.text()).toContain('项目管理系统');
    expect(card.text()).toContain('张三');
    expect(card.text()).toContain('2026-06-10');
    expect(card.text()).toContain('高');
    expect(card.text()).toContain('已延期');
  });

  test('emits open-task when card is clicked', async () => {
    const wrapper = mount(TaskKanbanView, {
      props: {
        tasks: [task({ id: 'task-1' })],
      },
      global: {
        stubs: {
          Draggable: draggableStub(),
        },
      },
    });

    await wrapper.get('[data-test="task-card-task-1"]').trigger('click');

    expect(wrapper.emitted('open-task')).toEqual([['task-1']]);
  });

  test('emits status-change when a card is dropped into another status column', () => {
    const movedTask = task({ id: 'task-1', status: 'todo' });
    const wrapper = mount(TaskKanbanView, {
      props: {
        tasks: [movedTask],
      },
      global: {
        stubs: {
          Draggable: draggableStub(),
        },
      },
    });

    wrapper.vm.handleColumnChange('done', { added: { element: movedTask } });

    expect(wrapper.emitted('status-change')).toEqual([[{ taskId: 'task-1', status: 'done' }]]);
  });
});

function task(overrides: Partial<TaskItem>): TaskItem {
  return {
    id: 'task-1',
    project_id: 'project-1',
    project_name: '项目管理系统',
    title: '需求文档确认',
    assignee_id: 'user-1',
    assignee_name: '张三',
    status: 'todo',
    priority: 'medium',
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
