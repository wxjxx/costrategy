CREATE TABLE task_subtasks (
    id uuid PRIMARY KEY,
    task_id uuid NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    assignee_id uuid NOT NULL REFERENCES users(id),
    status text NOT NULL DEFAULT 'todo' CHECK (status IN ('todo', 'in_progress', 'blocked', 'done')),
    description text NOT NULL CHECK (length(trim(description)) > 0 AND length(description) <= 1000),
    completed_is_overdue boolean,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_task_subtasks_task ON task_subtasks (task_id, created_at);
CREATE INDEX idx_task_subtasks_assignee ON task_subtasks (assignee_id);
