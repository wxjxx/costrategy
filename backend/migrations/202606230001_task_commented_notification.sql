ALTER TABLE notification_rules
    DROP CONSTRAINT notification_rules_rule_type_check;

ALTER TABLE notification_rules
    ADD CONSTRAINT notification_rules_rule_type_check
    CHECK (rule_type IN (
        'task_assigned',
        'assignee_changed',
        'task_commented',
        'due_tomorrow',
        'task_overdue'
    ));

ALTER TABLE notification_records
    DROP CONSTRAINT notification_records_notification_type_check;

ALTER TABLE notification_records
    ADD CONSTRAINT notification_records_notification_type_check
    CHECK (notification_type IN (
        'task_assigned',
        'assignee_changed',
        'task_commented',
        'due_tomorrow',
        'task_overdue'
    ));
