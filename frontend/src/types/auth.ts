export type UserRole = 'employee' | 'manager' | 'admin';

export interface CurrentUser {
  id: string;
  name: string;
  role: UserRole;
  departments: string[];
  permissions: string[];
}
