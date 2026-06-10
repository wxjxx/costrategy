CREATE TABLE IF NOT EXISTS task_assignees (
    task_id uuid NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES users(id),
    position integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (task_id, user_id)
);

INSERT INTO task_assignees (task_id, user_id, position)
SELECT id, assignee_id, 0
FROM tasks
ON CONFLICT (task_id, user_id) DO NOTHING;

CREATE INDEX IF NOT EXISTS idx_task_assignees_user ON task_assignees (user_id, task_id);
