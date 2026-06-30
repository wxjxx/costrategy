<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useQuery } from "@tanstack/vue-query";
import { useRoute, useRouter } from "vue-router";
import { useEditor, EditorContent } from "@tiptap/vue-3";
import StarterKit from "@tiptap/starter-kit";
import Link from "@tiptap/extension-link";
import Image from "@tiptap/extension-image";
import Underline from "@tiptap/extension-underline";
import Table from "@tiptap/extension-table";
import TableRow from "@tiptap/extension-table-row";
import TableCell from "@tiptap/extension-table-cell";
import TableHeader from "@tiptap/extension-table-header";
import TaskList from "@tiptap/extension-task-list";
import TaskItem from "@tiptap/extension-task-item";
import {
  Calendar,
  ChatLineRound,
  Delete,
  Document,
  Download,
  Grid,
  Link as LinkIcon,
  Picture,
  RefreshLeft,
  RefreshRight,
  Upload,
} from "@element-plus/icons-vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { useMutation, useQueryClient } from "@tanstack/vue-query";
import { api } from "@/api/client";
import PriorityTag from "@/components/PriorityTag.vue";
import StatusTag from "@/components/StatusTag.vue";
import UserAvatar from "@/components/UserAvatar.vue";
import { selectableUsers } from "@/features/users/userOptions";
import { clampPage, pageRows } from "@/utils/pagination";
import type { TaskStatus, TaskSubtask } from "@/types";
import { isSvgFile } from "./fileValidation";
import { renderDescriptionHtml } from "./richText";

const route = useRoute();
const router = useRouter();
const queryClient = useQueryClient();
const taskId = computed(() =>
  route.params.id ? String(route.params.id) : undefined,
);
const isNew = computed(() => route.name === "task-new");

const { data: detail } = useQuery({
  queryKey: ["task-detail-form", taskId],
  queryFn: () => api.taskDetail(taskId.value ?? ""),
  enabled: computed(() => Boolean(taskId.value)),
});
const { data: projects } = useQuery({ queryKey: ["projects"], queryFn: api.projects });
const { data: users } = useQuery({ queryKey: ["users"], queryFn: api.users });
const assigneeOptions = computed(() => selectableUsers(users.value ?? []));
const uploadInput = ref<HTMLInputElement>();
const editorImageInput = ref<HTMLInputElement>();
const pendingFiles = ref<File[]>([]);
const attachmentPage = ref(1);
const attachmentPageSize = ref(10);
const originalDueDate = ref("");
const attachmentRows = computed(() => [
  ...(detail.value?.attachments ?? []).map((attachment) => ({
    id: attachment.id,
    file_name: attachment.file_name,
    uploader_name: attachment.uploader_name ?? "-",
    created_at: attachment.created_at,
    pending: false,
  })),
  ...pendingFiles.value.map((file, index) => ({
    id: `pending-${index}-${file.name}`,
    file_name: file.name,
    uploader_name: "待上传",
    created_at: `${Math.max(1, Math.round(file.size / 1024))} KB`,
    pending: true,
    pendingIndex: index,
  })),
]);
const pagedAttachmentRows = computed(() =>
  pageRows(attachmentRows.value, attachmentPage.value, attachmentPageSize.value),
);

const form = ref({
  title: "",
  project_id: "",
  assignee_ids: [] as string[],
  status: "todo",
  priority: "medium",
  start_date: "",
  due_date: "",
});
const subtaskDialogVisible = ref(false);
const editingSubtaskId = ref("");
const subtaskForm = ref({
  assignee_id: "",
  status: "todo" as TaskStatus,
  description: "",
});
const subtasks = computed(() => detail.value?.task.subtasks ?? []);
type EditableSubtask = TaskSubtask & { pending?: boolean };
const pendingSubtasks = ref<EditableSubtask[]>([]);
const subtaskRows = computed<EditableSubtask[]>(() =>
  isNew.value ? pendingSubtasks.value : subtasks.value,
);
let pendingSubtaskSequence = 0;

function userAvatar(userId?: string): string | undefined {
  return users.value?.find((user) => user.id === userId)?.avatar_url;
}

const editor = useEditor({
  extensions: [
    StarterKit.configure({
      heading: { levels: [1, 2, 3] },
    }),
    Underline,
    Link.configure({ openOnClick: false }),
    Image,
    Table.configure({ resizable: false }),
    TableRow,
    TableHeader,
    TableCell,
    TaskList,
    TaskItem.configure({ nested: true }),
  ],
  content: "",
  editorProps: {
    attributes: {
      class: "tiptap-editor",
    },
    handlePaste(_view, event) {
      const selectedFiles = Array.from(event.clipboardData?.files ?? []);
      const imageFiles = selectedFiles.filter(isAllowedRichTextImage);
      if (imageFiles.length === 0) return false;
      if (selectedFiles.length > imageFiles.length) {
        ElMessage.warning("不支持上传 SVG 图片");
      }
      event.preventDefault();
      void uploadEditorImages(imageFiles);
      return true;
    },
  },
});

watch(
  () => detail.value?.task,
  (task) => {
    if (!task || isNew.value) return;
    form.value = {
      title: task.title,
      project_id: task.project_id,
      assignee_ids: task.assignees?.map((assignee) => assignee.id) ?? [task.assignee_id],
      status: task.status,
      priority: task.priority,
      start_date: task.start_date,
      due_date: task.due_date,
    };
    originalDueDate.value = task.due_date;
    editor.value?.commands.setContent(renderDescriptionHtml(task.description_json));
  },
  { immediate: true },
);

watch(
  () => attachmentRows.value.length,
  (total) => {
    attachmentPage.value = clampPage(attachmentPage.value, total, attachmentPageSize.value);
  },
);

watch(attachmentPageSize, () => {
  attachmentPage.value = 1;
});

onBeforeUnmount(() => {
  editor.value?.destroy();
});

const saveMutation = useMutation({
  mutationFn: async (dueDateChangeReason?: string) => {
    const payload = {
      ...form.value,
      assignee_id: form.value.assignee_ids[0] ?? "",
      status: form.value.status as "todo" | "in_progress" | "blocked" | "done",
      priority: form.value.priority as "low" | "medium" | "high",
      description_json: editor.value?.getJSON() ?? {},
      due_date_change_reason: dueDateChangeReason,
    };
    const task = await (taskId.value
      ? api.updateTask(taskId.value, payload)
      : api.createTask(payload));
    if (!taskId.value) {
      for (const subtask of pendingSubtasks.value) {
        await api.createSubtask(task.id, {
          assignee_id: subtask.assignee_id,
          status: subtask.status,
          description: subtask.description,
        });
      }
    }
    for (const file of pendingFiles.value) {
      await api.uploadTaskAttachment(task.id, file);
    }
    return task;
  },
  onSuccess: (task) => {
    pendingFiles.value = [];
    pendingSubtasks.value = [];
    ElMessage.success("任务已保存");
    void queryClient.invalidateQueries({ queryKey: ["tasks"] });
    void queryClient.invalidateQueries({ queryKey: ["task-detail"] });
    void router.push(`/tasks/${task.id}`);
  },
});

const attachmentDeleteMutation = useMutation({
  mutationFn: (attachmentId: string) =>
    api.deleteTaskAttachment(taskId.value ?? "", attachmentId),
  onSuccess: () => {
    ElMessage.success("附件已删除");
    void queryClient.invalidateQueries({ queryKey: ["task-detail-form", taskId] });
    void queryClient.invalidateQueries({ queryKey: ["task-detail"] });
  },
  onError: () => ElMessage.error("附件删除失败，请查看后端日志"),
});

const subtaskSaveMutation = useMutation({
  mutationFn: () => {
    const payload = { ...subtaskForm.value };
    if (editingSubtaskId.value) {
      return api.updateSubtask(taskId.value ?? "", editingSubtaskId.value, payload);
    }
    return api.createSubtask(taskId.value ?? "", payload);
  },
  onSuccess: () => {
    ElMessage.success("子任务已保存");
    subtaskDialogVisible.value = false;
    refreshSubtaskQueries();
  },
  onError: () => ElMessage.error("子任务保存失败，请查看后端日志"),
});

const subtaskDeleteMutation = useMutation({
  mutationFn: (subtaskId: string) => api.deleteSubtask(taskId.value ?? "", subtaskId),
  onSuccess: () => {
    ElMessage.success("子任务已删除");
    refreshSubtaskQueries();
  },
  onError: () => ElMessage.error("子任务删除失败，请查看后端日志"),
});

function refreshSubtaskQueries() {
  void queryClient.invalidateQueries({ queryKey: ["tasks"] });
  void queryClient.invalidateQueries({ queryKey: ["task-detail"] });
  void queryClient.invalidateQueries({ queryKey: ["task-detail-form", taskId] });
}

async function saveTask() {
  if (!isNew.value && originalDueDate.value && form.value.due_date !== originalDueDate.value) {
    try {
      const { value } = await ElMessageBox.prompt(
        "请输入截止日期变更原因",
        "截止日期变更",
        {
          inputType: "textarea",
          inputPlaceholder: "请说明为什么调整截止日期",
          inputValidator: (value) => Boolean(String(value ?? "").trim()),
          inputErrorMessage: "请填写变更原因",
          confirmButtonText: "继续保存",
          cancelButtonText: "取消",
        },
      );
      saveMutation.mutate(String(value ?? "").trim());
    } catch {
      // 用户取消填写时不保存。
    }
    return;
  }
  saveMutation.mutate(undefined);
}

async function setLink() {
  const currentEditor = editor.value;
  if (!currentEditor) return;
  const currentHref = editor.value?.getAttributes("link").href as string | undefined;
  try {
    const { value } = await ElMessageBox.prompt("请输入链接地址，留空可取消当前链接", "链接", {
      inputValue: currentHref ?? "https://",
      inputPlaceholder: "https://example.com",
      confirmButtonText: "确定",
      cancelButtonText: "取消",
    });
    const href = String(value ?? "").trim();
    if (!href) {
      currentEditor.chain().focus().unsetLink().run();
      return;
    }
    const { from, to, empty } = currentEditor.state.selection;
    const inserted = empty
      ? currentEditor
          .chain()
          .focus()
          .insertContent([
            {
              type: "text",
              text: href,
              marks: [{ type: "link", attrs: { href } }],
            },
            { type: "text", text: " " },
          ])
          .run()
      : currentEditor.chain().focus().setTextSelection({ from, to }).setLink({ href }).run();
    if (inserted) ElMessage.success("已插入链接");
  } catch {
    // 用户取消输入时不修改编辑器内容。
  }
}

function selectEditorImage() {
  editorImageInput.value?.click();
}

function uploadSelectedEditorImage(event: Event) {
  const input = event.target as HTMLInputElement;
  const selectedFiles = Array.from(input.files ?? []);
  const files = selectedFiles.filter(isAllowedRichTextImage);
  if (selectedFiles.length > files.length) {
    ElMessage.warning("不支持上传 SVG 图片");
  }
  if (files.length > 0) void uploadEditorImages(files);
  input.value = "";
}

async function uploadEditorImages(files: File[]) {
  for (const file of files) {
    await uploadEditorImage(file);
  }
}

async function uploadEditorImage(file: File) {
  const currentEditor = editor.value;
  if (!currentEditor) return;
  try {
    const { url } = await api.uploadRichTextImage(file);
    const inserted = currentEditor.chain().focus().setImage({ src: url }).run();
    if (inserted) {
      ElMessage.success("图片已上传并插入");
    } else {
      ElMessage.warning("图片插入失败，请先点击编辑区域后重试");
    }
  } catch {
    ElMessage.error("图片上传失败，请查看后端日志");
  }
}

function isAllowedRichTextImage(file: File): boolean {
  return file.type.startsWith("image/") && !isSvgFile(file);
}

function insertTable() {
  const currentEditor = editor.value;
  if (!currentEditor) return;
  const insertAt = currentEditor.state.doc.content.size;
  const inserted = currentEditor
    .chain()
    .focus()
    .setTextSelection(insertAt)
    .insertTable({ rows: 3, cols: 3, withHeaderRow: true })
    .run();
  if (inserted) {
    ElMessage.success("已插入表格");
  } else {
    ElMessage.warning("表格插入失败，请先点击编辑区域后重试");
  }
}

function deleteTable() {
  const deleted = editor.value?.chain().focus().deleteTable().run();
  if (deleted) ElMessage.success("已删除表格");
}

function selectAttachmentFile() {
  uploadInput.value?.click();
}

function addPendingFiles(event: Event) {
  const input = event.target as HTMLInputElement;
  const selectedFiles = Array.from(input.files ?? []);
  const files = selectedFiles.filter((file) => !isSvgFile(file));
  if (selectedFiles.length > files.length) {
    ElMessage.warning("不支持上传 SVG 文件");
  }
  if (files.length > 0) {
    pendingFiles.value = [...pendingFiles.value, ...files];
    ElMessage.success(`已选择 ${files.length} 个附件，保存任务后上传`);
  }
  input.value = "";
}

function removePendingFile(index: number) {
  pendingFiles.value = pendingFiles.value.filter((_, currentIndex) => currentIndex !== index);
}

function subtaskAssigneeName(assigneeId: string): string | undefined {
  return assigneeOptions.value.find((user) => user.id === assigneeId)?.name;
}

async function downloadAttachment(attachmentId: string, fileName: string) {
  if (!taskId.value) return;
  try {
    const blob = await api.downloadTaskAttachment(taskId.value, attachmentId);
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = fileName;
    link.click();
    URL.revokeObjectURL(url);
  } catch {
    ElMessage.error("附件下载失败，请查看后端日志");
  }
}

async function deleteAttachment(attachmentId: string, fileName: string) {
  try {
    await ElMessageBox.confirm(`确认删除附件“${fileName}”？`, "删除附件", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
    });
    attachmentDeleteMutation.mutate(attachmentId);
  } catch {
    // 用户取消删除。
  }
}

function openSubtaskDialog(subtask?: EditableSubtask) {
  editingSubtaskId.value = subtask?.id ?? "";
  subtaskForm.value = {
    assignee_id: subtask?.assignee_id ?? "",
    status: subtask?.status ?? "todo",
    description: subtask?.description ?? "",
  };
  subtaskDialogVisible.value = true;
}

function saveSubtask() {
  if (!subtaskForm.value.assignee_id || !subtaskForm.value.description.trim()) {
    ElMessage.warning("请填写子任务负责人和描述");
    return;
  }
  subtaskForm.value.description = subtaskForm.value.description.trim();
  if (isNew.value) {
    const existingSubtask = pendingSubtasks.value.find(
      (subtask) => subtask.id === editingSubtaskId.value,
    );
    if (existingSubtask) {
      existingSubtask.assignee_id = subtaskForm.value.assignee_id;
      existingSubtask.assignee_name = subtaskAssigneeName(subtaskForm.value.assignee_id);
      existingSubtask.status = subtaskForm.value.status;
      existingSubtask.description = subtaskForm.value.description;
    } else {
      pendingSubtaskSequence += 1;
      pendingSubtasks.value = [
        ...pendingSubtasks.value,
        {
          id: `pending-subtask-${pendingSubtaskSequence}`,
          task_id: "",
          assignee_id: subtaskForm.value.assignee_id,
          assignee_name: subtaskAssigneeName(subtaskForm.value.assignee_id),
          status: subtaskForm.value.status,
          description: subtaskForm.value.description,
          updated_at: "待保存",
          is_overdue: false,
          display_status: subtaskForm.value.status,
          pending: true,
        },
      ];
    }
    subtaskDialogVisible.value = false;
    ElMessage.success("子任务已添加，保存任务后生效");
    return;
  }
  if (!taskId.value) return;
  subtaskSaveMutation.mutate();
}

function completeSubtask(subtask: EditableSubtask) {
  if (subtask.status === "done") return;
  if (isNew.value) {
    pendingSubtasks.value = pendingSubtasks.value.map((current) =>
      current.id === subtask.id ? { ...current, status: "done", display_status: "done" } : current,
    );
    ElMessage.success("子任务已标记完成，保存任务后生效");
    return;
  }
  subtaskForm.value = {
    assignee_id: subtask.assignee_id,
    status: "done",
    description: subtask.description,
  };
  editingSubtaskId.value = subtask.id;
  subtaskSaveMutation.mutate();
}

async function deleteSubtask(subtask: EditableSubtask) {
  if (isNew.value) {
    pendingSubtasks.value = pendingSubtasks.value.filter((current) => current.id !== subtask.id);
    ElMessage.success("子任务已移除");
    return;
  }
  try {
    await ElMessageBox.confirm(`确认删除子任务“${subtask.description}”？`, "删除子任务", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
    });
    subtaskDeleteMutation.mutate(subtask.id);
  } catch {
    // 用户取消删除。
  }
}
</script>

<template>
  <div class="task-form-page">
    <section class="content-card">
      <ElForm label-position="left" label-width="104px" class="task-form">
        <ElRow class="task-basic-grid" :gutter="24">
          <ElCol :xs="24" :sm="12" :lg="8">
            <ElFormItem label="任务标题：" required>
              <ElInput v-model="form.title" />
            </ElFormItem>
          </ElCol>
          <ElCol :xs="24" :sm="12" :lg="8">
            <ElFormItem label="所属项目：" required>
              <ElSelect v-model="form.project_id" filterable placeholder="请选择项目">
                <ElOption
                  v-for="project in projects ?? []"
                  :key="project.id"
                  :label="project.name"
                  :value="project.id"
                />
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :xs="24" :sm="12" :lg="8">
            <ElFormItem label="负责人：" required>
              <ElSelect
                v-model="form.assignee_ids"
                multiple
                filterable
                collapse-tags
                collapse-tags-tooltip
                placeholder="请选择负责人"
              >
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
          </ElCol>
          <ElCol :xs="24" :sm="12" :lg="8">
            <ElFormItem label="状态：" required>
              <ElSelect v-model="form.status">
                <ElOption label="待开始" value="todo" />
                <ElOption label="进行中" value="in_progress" />
                <ElOption label="阻塞" value="blocked" />
                <ElOption label="已完成" value="done" />
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :xs="24" :sm="12" :lg="8">
            <ElFormItem label="优先级：" required>
              <ElSelect v-model="form.priority">
                <ElOption label="高" value="high"><PriorityTag priority="high" /></ElOption>
                <ElOption label="中" value="medium"><PriorityTag priority="medium" /></ElOption>
                <ElOption label="低" value="low"><PriorityTag priority="low" /></ElOption>
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :xs="24" :sm="12" :lg="8">
            <ElFormItem label="开始日期：" required>
              <ElDatePicker v-model="form.start_date" value-format="YYYY-MM-DD">
                <template #prefix><ElIcon><Calendar /></ElIcon></template>
              </ElDatePicker>
            </ElFormItem>
          </ElCol>
          <ElCol :xs="24" :sm="12" :lg="8">
            <ElFormItem label="截止日期：" required>
              <ElDatePicker v-model="form.due_date" value-format="YYYY-MM-DD" />
            </ElFormItem>
          </ElCol>
        </ElRow>

        <ElFormItem label="任务描述：">
          <div class="editor-shell">
            <div class="editor-toolbar">
              <button type="button" :class="{ active: editor?.isActive('paragraph') }" @click="editor?.chain().focus().setParagraph().run()">段落</button>
              <button type="button" :class="{ active: editor?.isActive('heading', { level: 1 }) }" @click="editor?.chain().focus().toggleHeading({ level: 1 }).run()">H1</button>
              <button type="button" :class="{ active: editor?.isActive('heading', { level: 2 }) }" @click="editor?.chain().focus().toggleHeading({ level: 2 }).run()">H2</button>
              <button type="button" :class="{ active: editor?.isActive('heading', { level: 3 }) }" @click="editor?.chain().focus().toggleHeading({ level: 3 }).run()">H3</button>
              <button type="button" :class="{ active: editor?.isActive('bold') }" @click="editor?.chain().focus().toggleBold().run()">B</button>
              <button type="button" :class="{ active: editor?.isActive('italic') }" @click="editor?.chain().focus().toggleItalic().run()">I</button>
              <button type="button" :class="{ active: editor?.isActive('underline') }" @click="editor?.chain().focus().toggleUnderline().run()">U</button>
              <button type="button" :class="{ active: editor?.isActive('strike') }" @click="editor?.chain().focus().toggleStrike().run()">S</button>
              <span class="toolbar-separator" />
              <button type="button" title="无序列表" :class="{ active: editor?.isActive('bulletList') }" @click="editor?.chain().focus().toggleBulletList().run()">☰</button>
              <button type="button" title="编号列表" :class="{ active: editor?.isActive('orderedList') }" @click="editor?.chain().focus().toggleOrderedList().run()">1.</button>
              <button type="button" title="任务清单" :class="{ active: editor?.isActive('taskList') }" @click="editor?.chain().focus().toggleTaskList().run()">☑</button>
              <span class="toolbar-separator" />
              <button type="button" title="链接" :class="{ active: editor?.isActive('link') }" @click="setLink"><ElIcon><LinkIcon /></ElIcon></button>
              <button type="button" title="图片" @click="selectEditorImage"><ElIcon><Picture /></ElIcon></button>
              <input ref="editorImageInput" class="hidden-file-input" type="file" accept="image/*" multiple @change="uploadSelectedEditorImage" />
              <button type="button" title="表格" @click="insertTable"><ElIcon><Grid /></ElIcon></button>
              <button type="button" title="前插一行" :disabled="!editor?.isActive('table')" @click="editor?.chain().focus().addRowBefore().run()">行↑</button>
              <button type="button" title="后插一行" :disabled="!editor?.isActive('table')" @click="editor?.chain().focus().addRowAfter().run()">行↓</button>
              <button type="button" title="前插一列" :disabled="!editor?.isActive('table')" @click="editor?.chain().focus().addColumnBefore().run()">列←</button>
              <button type="button" title="后插一列" :disabled="!editor?.isActive('table')" @click="editor?.chain().focus().addColumnAfter().run()">列→</button>
              <button type="button" title="删除表格" :disabled="!editor?.isActive('table')" @click="deleteTable">删表</button>
              <button type="button" title="代码块" :class="{ active: editor?.isActive('codeBlock') }" @click="editor?.chain().focus().toggleCodeBlock().run()"><ElIcon><Document /></ElIcon></button>
              <button type="button" title="引用" :class="{ active: editor?.isActive('blockquote') }" @click="editor?.chain().focus().toggleBlockquote().run()"><ElIcon><ChatLineRound /></ElIcon></button>
              <span class="toolbar-separator" />
              <button type="button" title="撤销" @click="editor?.chain().focus().undo().run()"><ElIcon><RefreshLeft /></ElIcon></button>
              <button type="button" title="重做" @click="editor?.chain().focus().redo().run()"><ElIcon><RefreshRight /></ElIcon></button>
            </div>
            <EditorContent :editor="editor" />
            <span class="tiptap-credit">由 Tiptap 驱动</span>
          </div>
        </ElFormItem>
      </ElForm>
    </section>

    <section class="content-card subtask-card">
      <div class="section-heading">
        <h3>子任务：</h3>
        <ElButton type="primary" plain @click="openSubtaskDialog()">新增子任务</ElButton>
      </div>
      <ElTable :data="subtaskRows">
        <ElTableColumn label="任务描述" min-width="260">
          <template #default="{ row }">
            <span class="task-title-cell">
              <span v-if="row.is_overdue" class="overdue-mark">已延期</span>
              {{ row.description }}
            </span>
          </template>
        </ElTableColumn>
        <ElTableColumn label="负责人" width="160">
          <template #default="{ row }">
            <span class="table-user"><UserAvatar :name="row.assignee_name" :src="userAvatar(row.assignee_id)" />{{ row.assignee_name || "-" }}</span>
          </template>
        </ElTableColumn>
        <ElTableColumn label="状态" width="130">
          <template #default="{ row }">
            <StatusTag :status="row.status" />
          </template>
        </ElTableColumn>
        <ElTableColumn label="是否延期" width="110">
          <template #default="{ row }">{{ row.is_overdue ? "是" : "否" }}</template>
        </ElTableColumn>
        <ElTableColumn label="操作" width="190">
          <template #default="{ row }">
            <ElButton
              v-if="row.status !== 'done'"
              link
              type="success"
              :loading="subtaskSaveMutation.isPending.value"
              @click="completeSubtask(row)"
            >
              完成
            </ElButton>
            <ElButton link type="primary" @click="openSubtaskDialog(row)">编辑</ElButton>
            <ElButton
              link
              type="danger"
              :loading="subtaskDeleteMutation.isPending.value"
              @click="deleteSubtask(row)"
            >
              删除
            </ElButton>
          </template>
        </ElTableColumn>
      </ElTable>
    </section>

    <section class="content-card attachment-card">
      <div class="section-heading">
        <h3>附件上传：</h3>
        <ElButton type="primary" plain @click="selectAttachmentFile"><ElIcon><Upload /></ElIcon>上传附件</ElButton>
        <input ref="uploadInput" class="hidden-file-input" type="file" multiple @change="addPendingFiles" />
      </div>
      <ElTable :data="pagedAttachmentRows">
        <ElTableColumn prop="file_name" label="文件名" />
        <ElTableColumn prop="uploader_name" label="上传人" width="170" />
        <ElTableColumn prop="created_at" label="上传时间" width="210" />
        <ElTableColumn label="操作" width="130">
          <template #default="{ row }">
            <ElButton
              v-if="!row.pending"
              link
              type="primary"
              @click="downloadAttachment(row.id, row.file_name)"
            >
              <ElIcon><Download /></ElIcon>
            </ElButton>
            <ElButton
              v-if="row.pending"
              link
              type="danger"
              @click="removePendingFile(row.pendingIndex)"
            >
              移除
            </ElButton>
            <ElButton
              v-else
              link
              type="danger"
              :loading="attachmentDeleteMutation.isPending.value"
              @click="deleteAttachment(row.id, row.file_name)"
            >
              <ElIcon><Delete /></ElIcon>
            </ElButton>
          </template>
        </ElTableColumn>
      </ElTable>
      <div class="table-footer">
        <ElPagination
          v-model:current-page="attachmentPage"
          v-model:page-size="attachmentPageSize"
          background
          layout="total, sizes, prev, pager, next, jumper"
          :page-sizes="[10, 20, 30, 50]"
          :total="attachmentRows.length"
        />
      </div>
    </section>

    <div class="page-actions">
      <ElButton @click="router.back()">取消</ElButton>
      <ElButton type="primary" :loading="saveMutation.isPending.value" @click="saveTask">保存</ElButton>
    </div>

    <ElDialog
      v-model="subtaskDialogVisible"
      :title="editingSubtaskId ? '编辑子任务' : '新增子任务'"
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
  </div>
</template>
