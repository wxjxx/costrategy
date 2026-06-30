ALTER TABLE tasks
    ADD COLUMN completed_is_overdue boolean;

UPDATE tasks
SET completed_is_overdue = (timezone('Asia/Shanghai', now())::date > due_date)
WHERE status = 'done';
