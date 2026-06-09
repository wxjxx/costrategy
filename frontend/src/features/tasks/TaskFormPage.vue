<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useQuery } from "@tanstack/vue-query";
import { useRoute, useRouter } from "vue-router";
import { useEditor, EditorContent } from "@tiptap/vue-3";
import StarterKit from "@tiptap/starter-kit";
import Link from "@tiptap/extension-link";
import Image from "@tiptap/extension-image";
import { Calendar, Delete, Download, Upload } from "@element-plus/icons-vue";
import { ElMessage } from "element-plus";
import { useMutation, useQueryClient } from "@tanstack/vue-query";
import { api } from "@/api/client";
import PriorityTag from "@/components/PriorityTag.vue";
import UserAvatar from "@/components/UserAvatar.vue";
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

const form = ref({
  title: "",
  project_id: "",
  assignee_id: "",
  status: "todo",
  priority: "medium",
  start_date: "",
  due_date: "",
});

const editor = useEditor({
  extensions: [StarterKit, Link, Image],
  content: "",
  editorProps: {
    attributes: {
      class: "tiptap-editor",
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
      assignee_id: task.assignee_id,
      status: task.status,
      priority: task.priority,
      start_date: task.start_date,
      due_date: task.due_date,
    };
    editor.value?.commands.setContent(renderDescriptionHtml(task.description_json));
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  editor.value?.destroy();
});

const saveMutation = useMutation({
  mutationFn: () => {
    const payload = {
      ...form.value,
      status: form.value.status as "todo" | "in_progress" | "done",
      priority: form.value.priority as "low" | "medium" | "high",
      description_json: editor.value?.getJSON() ?? {},
    };
    return taskId.value
      ? api.updateTask(taskId.value, payload)
      : api.createTask(payload);
  },
  onSuccess: (task) => {
    ElMessage.success("任务已保存");
    void queryClient.invalidateQueries({ queryKey: ["tasks"] });
    void router.push(`/tasks/${task.id}`);
  },
});

function saveTask() {
  saveMutation.mutate();
}
</script>

<template>
  <div class="task-form-page">
    <section class="content-card">
      <ElForm label-position="left" label-width="92px" class="task-form">
        <ElRow :gutter="34">
          <ElCol :span="8">
            <ElFormItem label="任务标题：" required>
              <ElInput v-model="form.title" />
            </ElFormItem>
          </ElCol>
          <ElCol :span="8">
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
          <ElCol :span="8">
            <ElFormItem label="负责人：" required>
              <ElSelect v-model="form.assignee_id">
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
          <ElCol :span="8">
            <ElFormItem label="状态：" required>
              <ElSelect v-model="form.status">
                <ElOption label="待开始" value="todo" />
                <ElOption label="进行中" value="in_progress" />
                <ElOption label="已完成" value="done" />
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :span="8">
            <ElFormItem label="优先级：" required>
              <ElSelect v-model="form.priority">
                <ElOption label="高" value="high"><PriorityTag priority="high" /></ElOption>
                <ElOption label="中" value="medium"><PriorityTag priority="medium" /></ElOption>
                <ElOption label="低" value="low"><PriorityTag priority="low" /></ElOption>
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :span="4">
            <ElFormItem label="开始日期：" required>
              <ElDatePicker v-model="form.start_date" value-format="YYYY-MM-DD">
                <template #prefix><ElIcon><Calendar /></ElIcon></template>
              </ElDatePicker>
            </ElFormItem>
          </ElCol>
          <ElCol :span="4">
            <ElFormItem label="截止日期：" required>
              <ElDatePicker v-model="form.due_date" value-format="YYYY-MM-DD" />
            </ElFormItem>
          </ElCol>
        </ElRow>

        <ElFormItem label="任务描述：">
          <div class="editor-shell">
            <div class="editor-toolbar">
              <button type="button" @click="editor?.chain().focus().toggleHeading({ level: 1 }).run()">H1</button>
              <button type="button" @click="editor?.chain().focus().toggleHeading({ level: 2 }).run()">H2</button>
              <button type="button" @click="editor?.chain().focus().toggleBold().run()">B</button>
              <button type="button" @click="editor?.chain().focus().toggleItalic().run()">I</button>
              <button type="button" @click="editor?.chain().focus().toggleBulletList().run()">列表</button>
              <button type="button" @click="editor?.chain().focus().toggleOrderedList().run()">编号</button>
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
        <ElButton type="primary" plain><ElIcon><Upload /></ElIcon>上传附件</ElButton>
      </div>
      <ElTable :data="detail?.attachments ?? []">
        <ElTableColumn prop="file_name" label="文件名" />
        <ElTableColumn prop="uploader_name" label="上传人" width="170" />
        <ElTableColumn prop="created_at" label="上传时间" width="210" />
        <ElTableColumn label="操作" width="130">
          <template #default>
            <ElButton link type="primary"><ElIcon><Download /></ElIcon></ElButton>
            <ElButton link type="danger"><ElIcon><Delete /></ElIcon></ElButton>
          </template>
        </ElTableColumn>
      </ElTable>
    </section>

    <div class="page-actions">
      <ElButton @click="router.back()">取消</ElButton>
      <ElButton type="primary" :loading="saveMutation.isPending.value" @click="saveTask">保存</ElButton>
    </div>
  </div>
</template>
