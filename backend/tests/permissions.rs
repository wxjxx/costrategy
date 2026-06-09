use costrategy_backend::auth::{Permission, UserRole};

#[test]
fn employee_permissions_match_first_version_scope() {
    let role = UserRole::Employee;

    assert!(role.has(Permission::ViewWorkbench));
    assert!(role.has(Permission::ViewAllUnarchivedTasks));
    assert!(role.has(Permission::UpdateOwnTaskStatus));
    assert!(role.has(Permission::CommentTask));
    assert!(role.has(Permission::UploadTaskAttachment));
    assert!(role.has(Permission::DeleteOwnAttachment));

    assert!(!role.has(Permission::CreateTask));
    assert!(!role.has(Permission::EditTaskCoreFields));
    assert!(!role.has(Permission::UpdateAnyTaskStatus));
    assert!(!role.has(Permission::ManageProjects));
    assert!(!role.has(Permission::ManageUsers));
    assert!(!role.has(Permission::ManageSystemSettings));
}

#[test]
fn manager_permissions_include_task_and_project_management() {
    let role = UserRole::Manager;

    assert!(role.has(Permission::ViewWorkbench));
    assert!(role.has(Permission::ViewAllUnarchivedTasks));
    assert!(role.has(Permission::UpdateOwnTaskStatus));
    assert!(role.has(Permission::UpdateAnyTaskStatus));
    assert!(role.has(Permission::CreateTask));
    assert!(role.has(Permission::EditTaskCoreFields));
    assert!(role.has(Permission::ArchiveTask));
    assert!(role.has(Permission::DeleteAnyAttachment));
    assert!(role.has(Permission::ManageProjects));

    assert!(!role.has(Permission::ManageUsers));
    assert!(!role.has(Permission::ManageSystemSettings));
}

#[test]
fn admin_permissions_include_system_administration() {
    let role = UserRole::Admin;

    assert!(role.has(Permission::ManageProjects));
    assert!(role.has(Permission::ManageUsers));
    assert!(role.has(Permission::ManageSystemSettings));
    assert!(role.has(Permission::RunDingtalkSync));
    assert!(role.has(Permission::ViewNotificationRecords));
}

#[test]
fn parses_roles_from_database_values() {
    assert_eq!("employee".parse::<UserRole>().unwrap(), UserRole::Employee);
    assert_eq!("manager".parse::<UserRole>().unwrap(), UserRole::Manager);
    assert_eq!("admin".parse::<UserRole>().unwrap(), UserRole::Admin);
    assert!("owner".parse::<UserRole>().is_err());
}
