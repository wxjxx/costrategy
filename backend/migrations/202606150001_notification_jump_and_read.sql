ALTER TABLE notification_records
    ADD COLUMN jump_url text,
    ADD COLUMN read_at timestamptz;
