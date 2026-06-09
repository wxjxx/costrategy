use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    Employee,
    Manager,
    Admin,
}

impl serde::Serialize for UserRole {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for UserRole {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    ViewWorkbench,
    ViewAllUnarchivedTasks,
    UpdateOwnTaskStatus,
    UpdateAnyTaskStatus,
    CommentTask,
    UploadTaskAttachment,
    DeleteOwnAttachment,
    DeleteAnyAttachment,
    CreateTask,
    EditTaskCoreFields,
    ArchiveTask,
    ManageProjects,
    ManageUsers,
    ManageSystemSettings,
    RunDingtalkSync,
    ViewNotificationRecords,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown user role: {0}")]
pub struct RoleParseError(String);

impl UserRole {
    pub fn has(self, permission: Permission) -> bool {
        match self {
            Self::Employee => employee_has(permission),
            Self::Manager => employee_has(permission) || manager_has(permission),
            Self::Admin => {
                employee_has(permission) || manager_has(permission) || admin_has(permission)
            }
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Employee => "employee",
            Self::Manager => "manager",
            Self::Admin => "admin",
        }
    }

    pub fn permission_codes(self) -> Vec<&'static str> {
        ALL_PERMISSIONS
            .iter()
            .copied()
            .filter(|permission| self.has(*permission))
            .map(Permission::as_str)
            .collect()
    }
}

const ALL_PERMISSIONS: &[Permission] = &[
    Permission::ViewWorkbench,
    Permission::ViewAllUnarchivedTasks,
    Permission::UpdateOwnTaskStatus,
    Permission::UpdateAnyTaskStatus,
    Permission::CommentTask,
    Permission::UploadTaskAttachment,
    Permission::DeleteOwnAttachment,
    Permission::DeleteAnyAttachment,
    Permission::CreateTask,
    Permission::EditTaskCoreFields,
    Permission::ArchiveTask,
    Permission::ManageProjects,
    Permission::ManageUsers,
    Permission::ManageSystemSettings,
    Permission::RunDingtalkSync,
    Permission::ViewNotificationRecords,
];

impl Permission {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ViewWorkbench => "view_workbench",
            Self::ViewAllUnarchivedTasks => "view_all_unarchived_tasks",
            Self::UpdateOwnTaskStatus => "update_own_task_status",
            Self::UpdateAnyTaskStatus => "update_any_task_status",
            Self::CommentTask => "comment_task",
            Self::UploadTaskAttachment => "upload_task_attachment",
            Self::DeleteOwnAttachment => "delete_own_attachment",
            Self::DeleteAnyAttachment => "delete_any_attachment",
            Self::CreateTask => "create_task",
            Self::EditTaskCoreFields => "edit_task_core_fields",
            Self::ArchiveTask => "archive_task",
            Self::ManageProjects => "manage_projects",
            Self::ManageUsers => "manage_users",
            Self::ManageSystemSettings => "manage_system_settings",
            Self::RunDingtalkSync => "run_dingtalk_sync",
            Self::ViewNotificationRecords => "view_notification_records",
        }
    }
}

impl FromStr for UserRole {
    type Err = RoleParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "employee" => Ok(Self::Employee),
            "manager" => Ok(Self::Manager),
            "admin" => Ok(Self::Admin),
            other => Err(RoleParseError(other.to_string())),
        }
    }
}

fn employee_has(permission: Permission) -> bool {
    matches!(
        permission,
        Permission::ViewWorkbench
            | Permission::ViewAllUnarchivedTasks
            | Permission::UpdateOwnTaskStatus
            | Permission::CommentTask
            | Permission::UploadTaskAttachment
            | Permission::DeleteOwnAttachment
    )
}

fn manager_has(permission: Permission) -> bool {
    matches!(
        permission,
        Permission::UpdateAnyTaskStatus
            | Permission::DeleteAnyAttachment
            | Permission::CreateTask
            | Permission::EditTaskCoreFields
            | Permission::ArchiveTask
            | Permission::ManageProjects
    )
}

fn admin_has(permission: Permission) -> bool {
    matches!(
        permission,
        Permission::ManageUsers
            | Permission::ManageSystemSettings
            | Permission::RunDingtalkSync
            | Permission::ViewNotificationRecords
    )
}
