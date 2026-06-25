import type { User } from "@/types";

export function selectableUsers(users: User[]): User[] {
  return users.filter((user) => user.status === "active");
}
