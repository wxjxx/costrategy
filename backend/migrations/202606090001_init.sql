CREATE TABLE users (
    id uuid PRIMARY KEY,
    dingtalk_user_id text NOT NULL UNIQUE,
    union_id text,
    name text NOT NULL,
    avatar_url text,
    mobile text,
    role text NOT NULL DEFAULT 'employee' CHECK (role IN ('employee', 'manager', 'admin')),
    status text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled')),
    last_synced_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE departments (
    id uuid PRIMARY KEY,
    dingtalk_dept_id bigint NOT NULL UNIQUE,
    parent_dingtalk_dept_id bigint,
    name text NOT NULL,
    order_no bigint,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE department_users (
    department_id uuid NOT NULL REFERENCES departments(id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (department_id, user_id)
);

CREATE TABLE projects (
    id uuid PRIMARY KEY,
    code text NOT NULL UNIQUE,
    name text NOT NULL,
    owner_id uuid REFERENCES users(id),
    description text,
    start_date date,
    end_date date,
    status text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'completed', 'paused', 'archived')),
    archived_at timestamptz,
    creator_id uuid REFERENCES users(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE tasks (
    id uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES projects(id),
    title text NOT NULL,
    assignee_id uuid NOT NULL REFERENCES users(id),
    status text NOT NULL DEFAULT 'todo' CHECK (status IN ('todo', 'in_progress', 'done')),
    priority text NOT NULL DEFAULT 'medium' CHECK (priority IN ('low', 'medium', 'high')),
    start_date date NOT NULL,
    due_date date NOT NULL,
    description_json jsonb NOT NULL DEFAULT '{}'::jsonb,
    creator_id uuid NOT NULL REFERENCES users(id),
    archived_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (start_date <= due_date)
);

CREATE TABLE task_comments (
    id uuid PRIMARY KEY,
    task_id uuid NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    author_id uuid NOT NULL REFERENCES users(id),
    content text NOT NULL CHECK (length(trim(content)) > 0 AND length(content) <= 2000),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE task_attachments (
    id uuid PRIMARY KEY,
    task_id uuid NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    file_name text NOT NULL,
    file_size bigint NOT NULL CHECK (file_size >= 0),
    mime_type text,
    bucket text NOT NULL,
    object_key text NOT NULL,
    uploader_id uuid NOT NULL REFERENCES users(id),
    deleted_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE task_activity_logs (
    id uuid PRIMARY KEY,
    task_id uuid NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    actor_id uuid REFERENCES users(id),
    action text NOT NULL CHECK (action IN (
        'task_created',
        'assignee_changed',
        'status_changed',
        'schedule_changed',
        'priority_changed',
        'task_archived',
        'attachment_uploaded',
        'attachment_deleted',
        'comment_created'
    )),
    before_value jsonb,
    after_value jsonb,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE notification_rules (
    id uuid PRIMARY KEY,
    rule_type text NOT NULL UNIQUE CHECK (rule_type IN (
        'task_assigned',
        'assignee_changed',
        'due_tomorrow',
        'task_overdue'
    )),
    enabled boolean NOT NULL DEFAULT true,
    updated_by uuid REFERENCES users(id),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE notification_records (
    id uuid PRIMARY KEY,
    notification_type text NOT NULL CHECK (notification_type IN (
        'task_assigned',
        'assignee_changed',
        'due_tomorrow',
        'task_overdue'
    )),
    receiver_id uuid NOT NULL REFERENCES users(id),
    task_id uuid REFERENCES tasks(id) ON DELETE SET NULL,
    content_summary text NOT NULL,
    status text NOT NULL CHECK (status IN ('success', 'failed')),
    failure_reason text,
    sent_at timestamptz NOT NULL DEFAULT now(),
    dedupe_date date
);

CREATE TABLE dingtalk_sync_logs (
    id uuid PRIMARY KEY,
    started_at timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz,
    status text NOT NULL CHECK (status IN ('running', 'success', 'failed')),
    created_users integer NOT NULL DEFAULT 0,
    updated_users integer NOT NULL DEFAULT 0,
    disabled_users integer NOT NULL DEFAULT 0,
    failure_reason text
);

CREATE TABLE system_settings (
    key text PRIMARY KEY,
    value_encrypted text,
    value_masked text,
    updated_by uuid REFERENCES users(id),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_notification_dedupe
    ON notification_records (notification_type, task_id, receiver_id, dedupe_date)
    WHERE dedupe_date IS NOT NULL;

CREATE INDEX idx_tasks_filters ON tasks (project_id, assignee_id, status, priority, start_date, due_date);
CREATE INDEX idx_tasks_unarchived ON tasks (archived_at) WHERE archived_at IS NULL;
CREATE INDEX idx_comments_task ON task_comments (task_id, created_at);
CREATE INDEX idx_attachments_task ON task_attachments (task_id, created_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_notification_records_time ON notification_records (sent_at DESC);
