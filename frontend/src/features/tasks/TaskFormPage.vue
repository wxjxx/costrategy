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
import UserAvatar from "@/components/UserAvatar.vue";
import { clampPage, pageRows } from "@/utils/pagination";
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
const uploadInput = ref<HTMLInputElement>();
const editorImageInput = ref<HTMLInputElement>();
const pendingFiles = ref<File[]>([]);
const attachmentPage = ref(1);
const attachmentPageSize = ref(10);
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
  ],
  content: "",
  editorProps: {
    attributes: {
      class: "tiptap-editor",
    },
    handlePaste(_view, event) {
      const imageFiles = Array.from(event.clipboardData?.files ?? []).filter((file) =>
        file.type.startsWith("image/"),
      );
      if (imageFiles.length === 0) return false;
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
  mutationFn: async () => {
    const payload = {
      ...form.value,
      assignee_id: form.value.assignee_ids[0] ?? "",
      status: form.value.status as "todo" | "in_progress" | "done",
      priority: form.value.priority as "low" | "medium" | "high",
      description_json: editor.value?.getJSON() ?? {},
    };
    const task = await (taskId.value
      ? api.updateTask(taskId.value, payload)
      : api.createTask(payload));
    for (const file of pendingFiles.value) {
      await api.uploadTaskAttachment(task.id, file);
    }
    return task;
  },
  onSuccess: (task) => {
    pendingFiles.value = [];
    ElMessage.success("任务已保存");
    void queryClient.invalidateQueries({ queryKey: ["tasks"] });
    void queryClient.invalidateQueries({ queryKey: ["task-detail"] });
    void router.push(`/tasks/${task.id}`);
  },
});

function saveTask() {
  saveMutation.mutate();
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
  const files = Array.from(input.files ?? []).filter((file) => file.type.startsWith("image/"));
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

function selectAttachmentFile() {
  uploadInput.value?.click();
}

function addPendingFiles(event: Event) {
  const input = event.target as HTMLInputElement;
  const files = Array.from(input.files ?? []);
  if (files.length > 0) {
    pendingFiles.value = [...pendingFiles.value, ...files];
    ElMessage.success(`已选择 ${files.length} 个附件，保存任务后上传`);
  }
  input.value = "";
}

function removePendingFile(index: number) {
  pendingFiles.value = pendingFiles.value.filter((_, currentIndex) => currentIndex !== index);
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
              <ElSelect v-model="form.project_id">
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
                collapse-tags
                collapse-tags-tooltip
                placeholder="请选择负责人"
              >
                <ElOption
                  v-for="user in users ?? []"
                  :key="user.id"
                  :label="user.name"
                  :value="user.id"
                >
                  <span class="option-user"><UserAvatar :name="user.name" />{{ user.name }}</span>
                </ElOption>
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :xs="24" :sm="12" :lg="8">
            <ElFormItem label="状态：" required>
              <ElSelect v-model="form.status">
                <ElOption label="待开始" value="todo" />
                <ElOption label="进行中" value="in_progress" />
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
              <span class="toolbar-separator" />
              <button type="button" title="链接" :class="{ active: editor?.isActive('link') }" @click="setLink"><ElIcon><LinkIcon /></ElIcon></button>
              <button type="button" title="图片" @click="selectEditorImage"><ElIcon><Picture /></ElIcon></button>
              <input ref="editorImageInput" class="hidden-file-input" type="file" accept="image/*" multiple @change="uploadSelectedEditorImage" />
              <button type="button" title="表格" @click="insertTable"><ElIcon><Grid /></ElIcon></button>
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
            <ElButton v-if="!row.pending" link type="primary"><ElIcon><Download /></ElIcon></ElButton>
            <ElButton
              v-if="row.pending"
              link
              type="danger"
              @click="removePendingFile(row.pendingIndex)"
            >
              移除
            </ElButton>
            <ElButton v-else link type="danger"><ElIcon><Delete /></ElIcon></ElButton>
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
  </div>
</template>
