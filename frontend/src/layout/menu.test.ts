import { describe, expect, test } from 'vitest';
import { getVisibleNavItems } from './menu';

describe('getVisibleNavItems', () => {
  test('employee only sees workbench', () => {
    expect(getVisibleNavItems('employee').map((item) => item.key)).toEqual(['workbench']);
  });

  test('manager sees workbench and projects', () => {
    expect(getVisibleNavItems('manager').map((item) => item.key)).toEqual([
      'workbench',
      'projects',
    ]);
  });

  test('admin sees every first version menu entry', () => {
    expect(getVisibleNavItems('admin').map((item) => item.key)).toEqual([
      'workbench',
      'projects',
      'users',
      'settings',
    ]);
  });
});
