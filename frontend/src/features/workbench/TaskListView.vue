<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useMutation, useQuery, useQueryClient } from "@tanstack/vue-query";
import { ElMessage, ElMessageBox } from "element-plus";
import { useRouter } from "vue-router";
import { api } from "@/api/client";
import OverdueBadge from "@/components/OverdueBadge.vue";
import PriorityTag from "@/components/PriorityTag.vue";
import StatusTag from "@/components/StatusTag.vue";
import UserAvatar from "@/components/UserAvatar.vue";
import { selectableUsers } from "@/features/users/userOptions";
import type { Task, TaskStatus } from "@/types";
import {
  primaryTaskAssigneeName,
  taskAssigneeNames,
} from "@/features/tasks/taskWorkflow";
import { clampPage, pageRows } from "@/utils/pagination";
import { buildTaskListRows, type TaskListRow } from "./taskList";

const props = defineProps<{ tasks: Task[] }>();

const router = useRouter();
const queryClient = useQueryClient();
const currentPage = ref(1);
const pageSize = ref(10);
const subtaskDialogVisible = ref(false);
const editingSubtask = ref<Extract<TaskListRow, { rowKind: "subtask" }>>();
const subtaskForm = ref({
  assignee_id: "",
  status: "todo" as TaskStatus,
  description: "",
});
const { data: users } = useQuery({ queryKey: ["users"], queryFn: api.users });
const assigneeOptions = computed(() => selectableUsers(users.value ?? []));
const sortedTasks = computed(() =>
  [...props.tasks].sort((left, right) => right.updated_at.localeCompare(left.updated_at)),
);
const taskRows = computed(() => buildTaskListRows(sortedTasks.value));
const pagedTasks = computed(() => pageRows(taskRows.value, currentPage.value, pageSize.value));

const deleteMutation = useMutation({
  mutationFn: (taskId: string) => api.deleteTask(taskId),
  onSuccess: () => ElMessage.success("任务已删除"),
  onError: () => ElMessage.error("只有任务创建人或管理人员可以删除任务"),
  onSettled: () => queryClient.invalidateQueries({ queryKey: ["tasks"] }),
});

const subtaskSaveMutation = useMutation({
  mutationFn: () => {
    if (!editingSubtask.value) throw new Error("missing subtask");
    return api.updateSubtask(
      editingSubtask.value.task_id,
      editingSubtask.value.subtask_id,
      { ...subtaskForm.value },
    );
  },
  onSuccess: () => {
    ElMessage.success("子任务已保存");
    subtaskDialogVisible.value = false;
    void queryClient.invalidateQueries({ queryKey: ["tasks"] });
  },
  onError: () => ElMessage.error("子任务保存失败，请查看后端日志"),
});

const subtaskDeleteMutation = useMutation({
  mutationFn: (row: Extract<TaskListRow, { rowKind: "subtask" }>) =>
    api.deleteSubtask(row.task_id, row.subtask_id),
  onSuccess: () => {
    ElMessage.success("子任务已删除");
    void queryClient.invalidateQueries({ queryKey: ["tasks"] });
  },
  onError: () => ElMessage.error("子任务删除失败，请查看后端日志"),
});

async function deleteTask(task: Task) {
  try {
    await ElMessageBox.confirm(`确认删除任务“${task.title}”？`, "删除任务", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
    });
    deleteMutation.mutate(task.id);
  } catch {
    // User cancelled.
  }
}

function openRow(row: TaskListRow) {
  if (row.rowKind === "subtask") return;
  void router.push(`/tasks/${row.id}`);
}

function rowAssigneeName(row: TaskListRow): string {
  if (row.rowKind === "subtask") return row.assignee_name || "-";
  return taskAssigneeNames(row);
}

function rowPrimaryAssigneeName(row: TaskListRow): string | undefined {
  if (row.rowKind === "subtask") return row.assignee_name;
  return primaryTaskAssigneeName(row);
}

function rowPrimaryAssigneeAvatar(row: TaskListRow): string | undefined {
  const userId = row.rowKind === "subtask" ? row.assignee_id : row.assignees?.[0]?.id ?? row.assignee_id;
  return users.value?.find((user) => user.id === userId)?.avatar_url;
}

function openSubtaskDialog(row: Extract<TaskListRow, { rowKind: "subtask" }>) {
  editingSubtask.value = row;
  subtaskForm.value = {
    assignee_id: row.assignee_id,
    status: row.status,
    description: row.title,
  };
  subtaskDialogVisible.value = true;
}

function saveSubtask() {
  if (!subtaskForm.value.assignee_id || !subtaskForm.value.description.trim()) {
    ElMessage.warning("请填写子任务负责人和描述");
    return;
  }
  subtaskForm.value.description = subtaskForm.value.description.trim();
  subtaskSaveMutation.mutate();
}

function completeSubtask(row: Extract<TaskListRow, { rowKind: "subtask" }>) {
  editingSubtask.value = row;
  subtaskForm.value = {
    assignee_id: row.assignee_id,
    status: "done",
    description: row.title,
  };
  subtaskSaveMutation.mutate();
}

async function deleteSubtask(row: Extract<TaskListRow, { rowKind: "subtask" }>) {
  try {
    await ElMessageBox.confirm(`确认删除子任务“${row.title}”？`, "删除子任务", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
    });
    subtaskDeleteMutation.mutate(row);
  } catch {
    // User cancelled.
  }
}

watch(
  () => props.tasks.length,
  (total) => {
    currentPage.value = clampPage(currentPage.value, total, pageSize.value);
  },
);

watch(pageSize, () => {
  currentPage.value = 1;
});
</script>

<template>
  <ElTable
    :data="pagedTasks"
    class="task-table"
    row-key="id"
    default-expand-all
    @row-click="openRow"
  >
    <ElTableColumn type="index" width="54" :index="(index: number) => (currentPage - 1) * pageSize + index + 1" />
    <ElTableColumn label="任务标题" min-width="170">
      <template #default="{ row }">
        <span class="task-title-cell">
          <OverdueBadge v-if="row.is_overdue" />
          {{ row.title }}
        </span>
      </template>
    </ElTableColumn>
    <ElTableColumn prop="project_name" label="所属项目" min-width="180" />
    <ElTableColumn label="负责人" width="130">
      <template #default="{ row }">
        <span class="table-user">
          <UserAvatar :name="rowPrimaryAssigneeName(row)" :src="rowPrimaryAssigneeAvatar(row)" :size="28" />
          {{ rowAssigneeName(row) }}
        </span>
      </template>
    </ElTableColumn>
    <ElTableColumn label="状态" sortable width="126">
      <template #default="{ row }">
        <StatusTag :status="row.status" />
      </template>
    </ElTableColumn>
    <ElTableColumn label="优先级" sortable width="112">
      <template #default="{ row }">
        <PriorityTag v-if="row.priority" :priority="row.priority" />
        <span v-else>-</span>
      </template>
    </ElTableColumn>
    <ElTableColumn prop="start_date" label="开始日期" width="136" />
    <ElTableColumn prop="due_date" label="截止日期" sortable width="136" />
    <ElTableColumn label="操作" width="190">
      <template #default="{ row }">
        <template v-if="row.rowKind === 'task'">
          <ElButton link type="primary" @click.stop="router.push(`/tasks/${row.id}/edit`)">编辑</ElButton>
          <ElButton link type="danger" @click.stop="deleteTask(row)">删除</ElButton>
        </template>
        <template v-else>
          <ElButton
            v-if="row.status !== 'done'"
            link
            type="success"
            :loading="subtaskSaveMutation.isPending.value"
            @click.stop="completeSubtask(row)"
          >
            完成
          </ElButton>
          <ElButton link type="primary" @click.stop="openSubtaskDialog(row)">编辑</ElButton>
          <ElButton
            link
            type="danger"
            :loading="subtaskDeleteMutation.isPending.value"
            @click.stop="deleteSubtask(row)"
          >
            删除
          </ElButton>
        </template>
      </template>
    </ElTableColumn>
  </ElTable>
  <div class="table-footer">
    <ElPagination
      v-model:current-page="currentPage"
      v-model:page-size="pageSize"
      background
      layout="total, sizes, prev, pager, next, jumper"
      :page-sizes="[10, 20, 30, 50]"
      :total="sortedTasks.length"
    />
  </div>

  <ElDialog
    v-model="subtaskDialogVisible"
    title="编辑子任务"
    width="520"
    class="form-dialog"
  >
    <ElForm label-width="88px">
      <ElFormItem label="负责人" required>
        <ElSelect v-model="subtaskForm.assignee_id" filterable placeholder="请选择负责人">
            <ElOption
              v-for="user in assigneeOptions"
              :key="user.id"
              :label="user.name"
              :value="user.id"
            >
              <span class="option-user"><UserAvatar :name="user.name" :src="user.avatar_url" />{{ user.name }}</span>
            </ElOption>
        </ElSelect>
      </ElFormItem>
      <ElFormItem label="状态" required>
        <ElSelect v-model="subtaskForm.status">
          <ElOption label="待开始" value="todo" />
          <ElOption label="进行中" value="in_progress" />
          <ElOption label="阻塞" value="blocked" />
          <ElOption label="已完成" value="done" />
        </ElSelect>
      </ElFormItem>
      <ElFormItem label="任务描述" required>
        <ElInput v-model="subtaskForm.description" type="textarea" :rows="4" />
      </ElFormItem>
    </ElForm>
    <template #footer>
      <ElButton @click="subtaskDialogVisible = false">取消</ElButton>
      <ElButton
        type="primary"
        :loading="subtaskSaveMutation.isPending.value"
        @click="saveSubtask"
      >
        保存
      </ElButton>
    </template>
  </ElDialog>
</template>
