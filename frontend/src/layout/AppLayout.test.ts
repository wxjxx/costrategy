import { mount, RouterLinkStub } from '@vue/test-utils';
import { describe, expect, test } from 'vitest';
import { createMemoryHistory, createRouter } from 'vue-router';
import AppLayout from './AppLayout.vue';
import type { CurrentUser } from '../types/auth';

describe('AppLayout', () => {
  test('renders current user and admin menus', async () => {
    const wrapper = await renderLayout(user('admin'));

    expect(wrapper.text()).toContain('项目管理系统');
    expect(wrapper.text()).toContain('管理员');
    expect(wrapper.text()).toContain('工作台');
    expect(wrapper.text()).toContain('项目管理');
    expect(wrapper.text()).toContain('用户管理');
    expect(wrapper.text()).toContain('系统设置');
    expect(wrapper.get('[data-test="view"]').text()).toBe('页面内容');
  });

  test('hides admin-only menus for employee', async () => {
    const wrapper = await renderLayout(user('employee'));
    const navLabels = wrapper.findAll('.app-nav__item').map((item) => item.text());

    expect(navLabels).toEqual(['工作台']);
  });
});

async function renderLayout(currentUser: CurrentUser) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/workbench', component: { template: '<div />' } }],
  });
  await router.push('/workbench');
  await router.isReady();

  return mount(AppLayout, {
    props: {
      currentUser,
    },
    global: {
      plugins: [router],
      stubs: {
        RouterLink: RouterLinkStub,
        RouterView: { template: '<section data-test="view">页面内容</section>' },
      },
    },
  });
}

function user(role: CurrentUser['role']): CurrentUser {
  return {
    id: 'user-1',
    name: role === 'admin' ? '管理员' : '员工',
    role,
  };
}
