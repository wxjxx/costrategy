<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import RichTextEditor from './RichTextEditor.vue';
import { getAttachmentDownloadUrl } from './taskApi';
import {
  TASK_PRIORITY_OPTIONS,
  TASK_STATUS_COLUMNS,
  extractTaskDescriptionText,
  taskActivityLabel,
  taskAssigneeLabel,
  taskPersonLabel,
  taskPriorityLabel,
  taskProjectLabel,
  taskStatusLabel,
  type TaskDetail,
  type TaskItem,
  type UpdateTaskPayload,
} from './taskModel';

const props = withDefaults(
  defineProps<{
    detail: TaskDetail | null;
    createMode?: boolean;
    initialEditing?: boolean;
    loading?: boolean;
    submitting?: boolean;
    savingTask?: boolean;
    uploadingAttachment?: boolean;
    deletingAttachmentId?: string;
  }>(),
  {
    createMode: false,
    initialEditing: false,
    loading: false,
    submitting: false,
    savingTask: false,
    uploadingAttachment: false,
    deletingAttachmentId: '',
  },
);

const emit = defineEmits<{
  close: [];
  'submit-comment': [content: string];
  'save-task': [payload: UpdateTaskPayload];
  'upload-attachment': [file: File];
  'delete-attachment': [attachmentId: string];
}>();

const commentContent = ref('');
const isEditing = ref(false);
const editForm = reactive<UpdateTaskPayload>({
  project_id: '',
  title: '',
  assignee_id: '',
  status: 'todo',
  priority: 'medium',
  start_date: '',
  due_date: '',
  description_json: emptyDescriptionDoc(),
});
const task = computed(() => props.detail?.task ?? null);
const descriptionText = computed(() =>
  props.detail ? extractTaskDescriptionText(props.detail.task.description_json) : '',
);

watch(
  task,
  (value, oldValue) => {
    if (value && isEditing.value && oldValue && value !== oldValue) {
      isEditing.value = false;
    }

    if (value && !isEditing.value) {
      resetEditForm(value);
    }

    if (value && props.initialEditing) {
      isEditing.value = true;
    }
  },
  { immediate: true },
);

watch(
  () => props.initialEditing,
  (initialEditing) => {
    if (initialEditing && task.value) {
      resetEditForm(task.value);
      isEditing.value = true;
    }
  },
);

function submitComment() {
  const content = commentContent.value.trim();
  if (!content) {
    return;
  }

  emit('submit-comment', content);
  commentContent.value = '';
}

function handleAttachmentChange(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) {
    return;
  }

  emit('upload-attachment', file);
  input.value = '';
}

function startEditing() {
  if (!task.value) {
    return;
  }

  resetEditForm(task.value);
  isEditing.value = true;
}

function cancelEditing() {
  if (props.createMode) {
    emit('close');
    return;
  }

  if (task.value) {
    resetEditForm(task.value);
  }
  isEditing.value = false;
}

function submitTaskEdit() {
  const title = editForm.title.trim();
  const projectId = editForm.project_id.trim();
  const assigneeId = editForm.assignee_id.trim();
  if (!title || !projectId || !assigneeId) {
    return;
  }

  emit('save-task', {
    ...editForm,
    project_id: projectId,
    title,
    assignee_id: assigneeId,
  });
}

function resetEditForm(value: TaskItem) {
  editForm.project_id = value.project_id;
  editForm.title = value.title;
  editForm.assignee_id = value.assignee_id;
  editForm.status = value.status;
  editForm.priority = value.priority;
  editForm.start_date = value.start_date;
  editForm.due_date = value.due_date;
  editForm.description_json = cloneDescription(value.description_json);
}

function cloneDescription(description: unknown): unknown {
  if (!description) {
    return emptyDescriptionDoc();
  }

  return JSON.parse(JSON.stringify(description));
}

function emptyDescriptionDoc() {
  return {
    type: 'doc',
    content: [{ type: 'paragraph', content: [] }],
  };
}
</script>

<template>
  <aside class="task-detail-panel" data-test="task-detail">
    <header>
      <div>
        <span v-if="task && !createMode" class="task-detail-panel__project">
          {{ taskProjectLabel(task) }}
        </span>
        <h2>{{ createMode ? '新增任务' : task?.title || '任务详情' }}</h2>
      </div>
      <button type="button" class="icon-text-button" @click="emit('close')">关闭</button>
    </header>

    <div v-if="loading" class="task-detail-panel__state" data-test="detail-loading">加载中</div>

    <div v-else-if="detail && task" class="task-detail-panel__body">
      <form
        v-if="isEditing"
        class="task-edit-form"
        data-test="task-edit-form"
        @submit.prevent="submitTaskEdit"
      >
        <section class="task-detail-section task-edit-section">
          <div class="task-edit-grid">
            <label>
              <span>任务标题</span>
              <input v-model="editForm.title" data-test="edit-title" required type="text" />
            </label>
            <label>
              <span>所属项目</span>
              <input
                v-model="editForm.project_id"
                data-test="edit-project-id"
                required
                type="text"
              />
            </label>
            <label>
              <span>负责人</span>
              <input
                v-model="editForm.assignee_id"
                data-test="edit-assignee-id"
                required
                type="text"
              />
            </label>
            <label>
              <span>状态</span>
              <select v-model="editForm.status" data-test="edit-status">
                <option
                  v-for="column in TASK_STATUS_COLUMNS"
                  :key="column.status"
                  :value="column.status"
                >
                  {{ column.label }}
                </option>
              </select>
            </label>
            <label>
              <span>优先级</span>
              <select v-model="editForm.priority" data-test="edit-priority">
                <option
                  v-for="option in TASK_PRIORITY_OPTIONS"
                  :key="option.priority"
                  :value="option.priority"
                >
                  {{ option.label }}
                </option>
              </select>
            </label>
            <label>
              <span>开始日期</span>
              <input v-model="editForm.start_date" data-test="edit-start-date" type="date" />
            </label>
            <label>
              <span>截止日期</span>
              <input v-model="editForm.due_date" data-test="edit-due-date" type="date" />
            </label>
          </div>
        </section>

        <section class="task-detail-section">
          <h3>任务描述</h3>
          <RichTextEditor v-model="editForm.description_json" :disabled="savingTask" />
        </section>

        <div class="task-edit-actions">
          <button type="button" class="secondary-button" :disabled="savingTask" @click="cancelEditing">
            取消
          </button>
          <button type="submit" class="primary-button" :disabled="savingTask">
            {{ savingTask ? '保存中' : createMode ? '创建任务' : '保存任务' }}
          </button>
        </div>
      </form>

      <template v-else>
        <dl>
          <div>
            <dt>负责人</dt>
            <dd>{{ taskAssigneeLabel(task) }}</dd>
          </div>
          <div>
            <dt>状态</dt>
            <dd>{{ taskStatusLabel(task.status) }}</dd>
          </div>
          <div>
            <dt>优先级</dt>
            <dd>{{ taskPriorityLabel(task.priority) }}</dd>
          </div>
          <div>
            <dt>排期</dt>
            <dd>{{ task.start_date }} 至 {{ task.due_date }}</dd>
          </div>
          <div>
            <dt>延期</dt>
            <dd>{{ task.is_overdue ? '已延期' : '未延期' }}</dd>
          </div>
        </dl>

        <section class="task-detail-section">
          <h3>任务描述</h3>
          <p class="task-description-text">{{ descriptionText || '暂无描述' }}</p>
        </section>

        <section class="task-detail-section">
          <div class="task-detail-section__heading">
            <h3>附件</h3>
            <label
              class="secondary-button attachment-upload-button"
              :class="{ 'attachment-upload-button--disabled': uploadingAttachment }"
            >
              {{ uploadingAttachment ? '上传中' : '上传文件' }}
              <input
                data-test="attachment-input"
                type="file"
                :disabled="uploadingAttachment"
                @change="handleAttachmentChange"
              />
            </label>
          </div>
          <ul v-if="detail.attachments.length > 0" class="task-attachment-list">
            <li v-for="attachment in detail.attachments" :key="attachment.id">
              <div class="task-attachment-list__main">
                <a
                  :href="getAttachmentDownloadUrl(task.id, attachment.id)"
                  target="_blank"
                  rel="noreferrer"
                >
                  {{ attachment.file_name }}
                </a>
                <span>
                  {{ taskPersonLabel(attachment.uploader_name, attachment.uploader_id) }} ·
                  {{ attachment.created_at.slice(0, 10) }}
                </span>
              </div>
              <button
                type="button"
                class="icon-text-button"
                :data-test="`delete-attachment-${attachment.id}`"
                :disabled="deletingAttachmentId === attachment.id"
                @click="emit('delete-attachment', attachment.id)"
              >
                {{ deletingAttachmentId === attachment.id ? '删除中' : '删除' }}
              </button>
            </li>
          </ul>
          <p v-else class="task-detail-empty">暂无附件</p>
        </section>

        <section class="task-detail-section">
          <h3>评论</h3>
          <ul v-if="detail.comments.length > 0" class="task-comment-list">
            <li v-for="comment in detail.comments" :key="comment.id">
              <div>
                <strong>{{ taskPersonLabel(comment.author_name, comment.author_id) }}</strong>
                <time>{{ comment.created_at.slice(0, 10) }}</time>
              </div>
              <p>{{ comment.content }}</p>
            </li>
          </ul>
          <p v-else class="task-detail-empty">暂无评论</p>

          <form class="comment-form" data-test="comment-form" @submit.prevent="submitComment">
            <textarea
              v-model="commentContent"
              data-test="comment-input"
              rows="3"
              maxlength="2000"
              placeholder="添加评论"
            />
            <button type="submit" class="primary-button" :disabled="submitting">
              {{ submitting ? '提交中' : '发送评论' }}
            </button>
          </form>
        </section>

        <section class="task-detail-section">
          <h3>操作记录</h3>
          <ul v-if="detail.activity_logs.length > 0" class="task-activity-list">
            <li v-for="log in detail.activity_logs" :key="log.id">
              <span>{{ taskActivityLabel(log.action) }}</span>
              <em>{{ taskPersonLabel(log.actor_name, log.actor_id) }}</em>
            </li>
          </ul>
          <p v-else class="task-detail-empty">暂无操作记录</p>
        </section>

        <button type="button" class="primary-button" data-test="edit-task-button" @click="startEditing">
          编辑任务
        </button>
      </template>
    </div>
  </aside>
</template>
