<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useMutation, useQueryClient } from "@tanstack/vue-query";
import { useQuery } from "@tanstack/vue-query";
import { useRoute, useRouter } from "vue-router";
import { Delete, Download, Edit, UploadFilled } from "@element-plus/icons-vue";
import { ElMessage, ElMessageBox } from "element-plus";
import VueOfficeDocx from "@vue-office/docx";
import VueOfficeExcel from "@vue-office/excel";
import VueOfficePdf from "@vue-office/pdf";
import VueOfficePptx from "@vue-office/pptx";
import "@vue-office/docx/lib/index.css";
import "@vue-office/excel/lib/index.css";
import { api } from "@/api/client";
import PriorityTag from "@/components/PriorityTag.vue";
import StatusTag from "@/components/StatusTag.vue";
import UserAvatar from "@/components/UserAvatar.vue";
import type { TaskAttachment } from "@/types";
import {
  getDisplayStatus,
  primaryTaskAssigneeName,
  taskAssigneeNames,
} from "@/features/tasks/taskWorkflow";
import { clampPage, pageRows } from "@/utils/pagination";
import { formatDateTimeInShanghai } from "@/utils/datetime";
import { activityActionLabel, activityStatusLabel } from "./activityLabels";
import { attachmentPreviewKind, type AttachmentPreviewKind } from "./attachmentPreview";
import { isSvgFile } from "./fileValidation";
import { renderDescriptionHtml } from "./richText";

const route = useRoute();
const router = useRouter();
const queryClient = useQueryClient();
const taskId = computed(() => String(route.params.id));
const commentContent = ref("");
const uploadInput = ref<HTMLInputElement>();
const previewImageUrl = ref("");
const attachmentPage = ref(1);
const attachmentPageSize = ref(10);
const attachmentPreviewVisible = ref(false);
const attachmentPreviewLoading = ref(false);
const attachmentPreview = ref<{
  id: string;
  fileName: string;
  kind: AttachmentPreviewKind;
  blob?: Blob;
  url?: string;
}>();
let attachmentPreviewRequestId = 0;

const { data: detail } = useQuery({
  queryKey: ["task-detail", taskId],
  queryFn: () => api.taskDetail(taskId.value),
});

const task = computed(() => detail.value?.task);
const descriptionHtml = computed(() =>
  task.value ? renderDescriptionHtml(task.value.description_json) : "",
);
const pagedAttachments = computed(() =>
  pageRows(detail.value?.attachments ?? [], attachmentPage.value, attachmentPageSize.value),
);
const attachmentPreviewComponent = computed(() => {
  switch (attachmentPreview.value?.kind) {
    case "docx":
      return VueOfficeDocx;
    case "excel":
      return VueOfficeExcel;
    case "pdf":
      return VueOfficePdf;
    case "pptx":
      return VueOfficePptx;
    default:
      return undefined;
  }
});

watch(
  () => detail.value?.attachments.length ?? 0,
  (total) => {
    attachmentPage.value = clampPage(attachmentPage.value, total, attachmentPageSize.value);
  },
);

watch(attachmentPageSize, () => {
  attachmentPage.value = 1;
});

watch(attachmentPreviewVisible, (visible) => {
  if (!visible) {
    attachmentPreviewRequestId += 1;
    revokeAttachmentPreviewUrl();
    attachmentPreview.value = undefined;
    attachmentPreviewLoading.value = false;
  }
});

onBeforeUnmount(() => {
  revokeAttachmentPreviewUrl();
});

function refreshTaskQueries() {
  void queryClient.invalidateQueries({ queryKey: ["task-detail"] });
  void queryClient.invalidateQueries({ queryKey: ["tasks"] });
}

const commentMutation = useMutation({
  mutationFn: () => api.createTaskComment(taskId.value, commentContent.value),
  onSuccess: () => {
    commentContent.value = "";
    ElMessage.success("评论已发布");
    refreshTaskQueries();
  },
  onError: () => ElMessage.error("评论发布失败，请查看后端日志"),
});

const uploadMutation = useMutation({
  mutationFn: (file: File) => api.uploadTaskAttachment(taskId.value, file),
  onSuccess: () => {
    ElMessage.success("附件已上传");
    refreshTaskQueries();
  },
  onError: () => ElMessage.error("附件上传失败，请查看后端日志"),
});

const taskDeleteMutation = useMutation({
  mutationFn: () => api.deleteTask(taskId.value),
  onSuccess: () => {
    ElMessage.success("任务已删除");
    refreshTaskQueries();
    router.push("/workbench");
  },
  onError: () => ElMessage.error("只有任务创建人或管理人员可以删除任务"),
});

const attachmentDeleteMutation = useMutation({
  mutationFn: (attachmentId: string) =>
    api.deleteTaskAttachment(taskId.value, attachmentId),
  onSuccess: () => {
    ElMessage.success("附件已删除");
    refreshTaskQueries();
  },
  onError: () => ElMessage.error("附件删除失败，请查看后端日志"),
});

async function downloadAttachment(attachmentId: string, fileName: string) {
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

async function previewAttachment(attachment: TaskAttachment) {
  const kind = attachmentPreviewKind(attachment.file_name, attachment.mime_type);
  if (!kind) {
    ElMessage.warning("仅支持预览 Word、Excel、PDF、PPT、图片文件，请下载查看");
    return;
  }

  const requestId = attachmentPreviewRequestId + 1;
  attachmentPreviewRequestId = requestId;
  revokeAttachmentPreviewUrl();
  attachmentPreview.value = {
    id: attachment.id,
    fileName: attachment.file_name,
    kind,
  };
  attachmentPreviewVisible.value = true;
  attachmentPreviewLoading.value = true;

  try {
    const blob = await api.downloadTaskAttachment(taskId.value, attachment.id);
    if (requestId !== attachmentPreviewRequestId) return;
    const url = kind === "image" ? URL.createObjectURL(blob) : undefined;
    attachmentPreview.value = {
      id: attachment.id,
      fileName: attachment.file_name,
      kind,
      blob,
      url,
    };
  } catch {
    if (requestId === attachmentPreviewRequestId) {
      attachmentPreviewVisible.value = false;
      ElMessage.error("附件预览失败，请下载查看");
    }
  } finally {
    if (requestId === attachmentPreviewRequestId) {
      attachmentPreviewLoading.value = false;
    }
  }
}

function revokeAttachmentPreviewUrl() {
  if (attachmentPreview.value?.url) {
    URL.revokeObjectURL(attachmentPreview.value.url);
  }
}

async function downloadPreviewAttachment() {
  if (!attachmentPreview.value) return;
  await downloadAttachment(attachmentPreview.value.id, attachmentPreview.value.fileName);
}

function selectUploadFile() {
  uploadInput.value?.click();
}

function uploadSelectedFile(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (file && isSvgFile(file)) {
    ElMessage.warning("不支持上传 SVG 文件");
  } else if (file) {
    uploadMutation.mutate(file);
  }
  input.value = "";
}

function submitComment() {
  if (!commentContent.value.trim()) {
    ElMessage.warning("请输入评论内容");
    return;
  }
  commentMutation.mutate();
}

function openRichContentImage(event: MouseEvent) {
  const target = event.target;
  if (!(target instanceof HTMLImageElement)) return;
  previewImageUrl.value = target.currentSrc || target.src;
}

function activityTitle(log: { action: string; after_value?: Record<string, unknown> }) {
  if (log.action === "schedule_changed" && typeof log.after_value?.due_date === "string") {
    return `截止日期调整为${log.after_value.due_date}`;
  }
  if (log.action !== "status_changed") return activityActionLabel(log.action);
  const targetStatus = activityStatusLabel(
    typeof log.after_value?.status === "string" ? log.after_value.status : undefined,
  );
  return targetStatus ? `更新状态为${targetStatus}` : activityActionLabel(log.action);
}

function activityReason(log: { action: string; after_value?: Record<string, unknown> }) {
  if (log.action !== "schedule_changed") return undefined;
  return typeof log.after_value?.reason === "string" ? log.after_value.reason : undefined;
}

async function deleteCurrentTask() {
  if (!task.value) return;
  try {
    await ElMessageBox.confirm(`确认删除任务“${task.value.title}”？`, "删除任务", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
    });
    taskDeleteMutation.mutate();
  } catch {
    // User cancelled.
  }
}
</script>

<template>
  <div v-if="task && detail" class="task-detail-page">
    <section class="content-card task-title-card">
      <div>
        <h2>{{ task.title }}</h2>
        <StatusTag :status="getDisplayStatus(task)" />
        <span v-if="task.is_overdue" class="overdue-mark">已延期</span>
        <PriorityTag :priority="task.priority" />
      </div>
      <div class="task-title-actions">
        <ElButton type="primary" link @click="router.push(`/tasks/${task.id}/edit`)">
          <ElIcon><Edit /></ElIcon>
          编辑
        </ElButton>
        <ElButton type="danger" link @click="deleteCurrentTask">
          <ElIcon><Delete /></ElIcon>
          删除
        </ElButton>
      </div>
    </section>

    <section class="content-card info-card">
      <h3>基本信息</h3>
      <div class="info-grid">
        <span>所属项目：{{ task.project_name }}</span>
        <span class="with-avatar">
          负责人：<UserAvatar :name="primaryTaskAssigneeName(task)" />{{ taskAssigneeNames(task) }}
        </span>
        <span>状态：<StatusTag :status="getDisplayStatus(task)" /></span>
        <span>优先级：<PriorityTag :priority="task.priority" /></span>
        <span>开始日期：{{ task.start_date }}</span>
        <span>截止日期：{{ task.due_date }}</span>
        <span>是否延期：{{ task.is_overdue ? "是" : "否" }}</span>
        <span>创建人：{{ task.creator_name || task.creator_id }}</span>
      </div>
    </section>

    <div class="detail-grid">
      <section class="left-stack">
        <article class="content-card">
          <h3>任务描述</h3>
          <div
            v-if="descriptionHtml"
            class="rich-content rich-content-detail"
            v-html="descriptionHtml"
            @click="openRichContentImage"
          />
          <p v-else class="empty-text">暂无任务描述</p>
        </article>

        <article class="content-card">
          <div class="section-heading">
            <h3>附件</h3>
            <ElButton
              type="primary"
              plain
              :loading="uploadMutation.isPending.value"
              @click="selectUploadFile"
            >
              <ElIcon><UploadFilled /></ElIcon>
              上传附件
            </ElButton>
          </div>
          <input ref="uploadInput" class="hidden-file-input" type="file" @change="uploadSelectedFile" />
          <ElTable :data="pagedAttachments" size="small">
            <ElTableColumn label="文件名" min-width="220">
              <template #default="{ row }">
                <ElButton
                  class="attachment-name-button"
                  link
                  type="primary"
                  @click="previewAttachment(row)"
                >
                  {{ row.file_name }}
                </ElButton>
              </template>
            </ElTableColumn>
            <ElTableColumn label="文件大小" width="120">
              <template #default="{ row }">{{ (row.file_size / 1024 / 1024).toFixed(2) }} MB</template>
            </ElTableColumn>
            <ElTableColumn prop="uploader_name" label="上传人" width="100" />
            <ElTableColumn prop="created_at" label="上传时间" width="170" />
            <ElTableColumn label="操作" width="120">
              <template #default="{ row }">
                <ElButton link type="primary" @click="downloadAttachment(row.id, row.file_name)">
                  <ElIcon><Download /></ElIcon>
                </ElButton>
                <ElButton link type="danger" @click="attachmentDeleteMutation.mutate(row.id)">
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
              :total="detail.attachments.length"
            />
          </div>
        </article>
      </section>

      <aside class="right-stack">
        <article class="content-card">
          <h3>评论</h3>
          <div v-for="comment in detail.comments" :key="comment.id" class="comment-row">
            <UserAvatar :name="comment.author_name" />
            <div>
              <strong>{{ comment.author_name }}</strong>
              <p>{{ comment.content }}</p>
            </div>
            <time>{{ formatDateTimeInShanghai(comment.created_at, false) }}</time>
          </div>
          <div class="comment-input">
            <ElInput v-model="commentContent" placeholder="请输入评论内容..." />
            <ElButton type="primary" :loading="commentMutation.isPending.value" @click="submitComment">发表评论</ElButton>
          </div>
        </article>

        <article class="content-card activity-card">
          <h3>操作记录</h3>
          <ElTimeline class="activity-timeline">
            <ElTimelineItem
              v-for="log in detail.activity_logs"
              :key="log.id"
              :timestamp="formatDateTimeInShanghai(log.created_at, false)"
            >
              <strong>{{ activityTitle(log) }}</strong>
              <p>{{ log.actor_name }}</p>
              <p v-if="activityReason(log)" class="activity-reason">原因：{{ activityReason(log) }}</p>
            </ElTimelineItem>
          </ElTimeline>
        </article>
      </aside>
    </div>

    <ElImageViewer
      v-if="previewImageUrl"
      :url-list="[previewImageUrl]"
      hide-on-click-modal
      @close="previewImageUrl = ''"
    />

    <ElDialog
      v-model="attachmentPreviewVisible"
      :title="attachmentPreview?.fileName ?? '附件预览'"
      width="88vw"
      top="4vh"
      destroy-on-close
      class="attachment-preview-dialog"
    >
      <div
        v-loading="attachmentPreviewLoading"
        class="attachment-preview-body"
        element-loading-text="正在加载附件..."
      >
        <component
          :is="attachmentPreviewComponent"
          v-if="attachmentPreview?.blob && attachmentPreviewComponent"
          :src="attachmentPreview.blob"
        />
        <ElImage
          v-else-if="attachmentPreview?.kind === 'image' && attachmentPreview.url"
          class="attachment-image-preview"
          :src="attachmentPreview.url"
          :preview-src-list="[attachmentPreview.url]"
          fit="contain"
          hide-on-click-modal
          preview-teleported
        />
      </div>
      <template #footer>
        <ElButton
          v-if="attachmentPreview"
          :loading="attachmentPreviewLoading"
          @click="downloadPreviewAttachment"
        >
          <ElIcon><Download /></ElIcon>
          下载
        </ElButton>
        <ElButton type="primary" @click="attachmentPreviewVisible = false">关闭</ElButton>
      </template>
    </ElDialog>
  </div>
</template>
