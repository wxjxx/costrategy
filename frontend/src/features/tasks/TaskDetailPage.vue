<script setup lang="ts">
import { computed } from "vue";
import { useQuery } from "@tanstack/vue-query";
import { useRoute, useRouter } from "vue-router";
import { Download, Edit, Refresh, UploadFilled } from "@element-plus/icons-vue";
import { api } from "@/api/client";
import PriorityTag from "@/components/PriorityTag.vue";
import StatusTag from "@/components/StatusTag.vue";
import UserAvatar from "@/components/UserAvatar.vue";
import { getDisplayStatus } from "@/features/tasks/taskWorkflow";
import { renderDescriptionHtml } from "./richText";

const route = useRoute();
const router = useRouter();
const taskId = computed(() => String(route.params.id));

const { data: detail } = useQuery({
  queryKey: ["task-detail", taskId],
  queryFn: () => api.taskDetail(taskId.value),
});

const task = computed(() => detail.value?.task);
const descriptionHtml = computed(() =>
  task.value ? renderDescriptionHtml(task.value.description_json) : "",
);
</script>

<template>
  <div v-if="task && detail" class="task-detail-page">
    <section class="content-card task-title-card">
      <div>
        <h2>{{ task.title }}</h2>
        <StatusTag :status="getDisplayStatus(task)" />
        <PriorityTag :priority="task.priority" />
      </div>
      <ElButton type="primary" link @click="router.push(`/tasks/${task.id}/edit`)">
        <ElIcon><Edit /></ElIcon>
        编辑
      </ElButton>
    </section>

    <section class="content-card info-card">
      <h3>基本信息</h3>
      <div class="info-grid">
        <span>所属项目：{{ task.project_name }}</span>
        <span class="with-avatar">负责人：<UserAvatar :name="task.assignee_name" />{{ task.assignee_name }}</span>
        <span>状态：<StatusTag :status="getDisplayStatus(task)" /></span>
        <span>优先级：<PriorityTag :priority="task.priority" /></span>
        <span>开始日期：{{ task.start_date }}</span>
        <span>截止日期：{{ task.due_date }}</span>
        <span>是否延期：{{ task.is_overdue && task.status !== "done" ? "是" : "否" }}</span>
        <span>创建人：{{ task.creator_id }}</span>
      </div>
    </section>

    <div class="detail-grid">
      <section class="left-stack">
        <article class="content-card">
          <h3>任务描述</h3>
          <div class="rich-content" v-html="descriptionHtml" />
          <div class="mini-gantt">
            <div class="mini-gantt-header">
              <span>项目：{{ task.project_name || "-" }}</span>
              <span>缩放：周</span>
              <span>{{ task.start_date }} ~ {{ task.due_date }}</span>
            </div>
            <div class="mini-gantt-body">
              <span v-for="index in 7" :key="index" :style="{ width: `${16 + index * 8}%` }" />
            </div>
          </div>
        </article>

        <article class="content-card">
          <h3>附件</h3>
          <ElTable :data="detail.attachments" size="small">
            <ElTableColumn prop="file_name" label="文件名" />
            <ElTableColumn label="文件大小" width="120">
              <template #default="{ row }">{{ (row.file_size / 1024 / 1024).toFixed(2) }} MB</template>
            </ElTableColumn>
            <ElTableColumn prop="mime_type" label="文件类型" width="100" />
            <ElTableColumn prop="uploader_name" label="上传人" width="100" />
            <ElTableColumn prop="created_at" label="上传时间" width="170" />
            <ElTableColumn label="操作" width="90">
              <template #default>
                <ElButton link type="primary"><ElIcon><Download /></ElIcon></ElButton>
              </template>
            </ElTableColumn>
          </ElTable>
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
            <time>{{ comment.created_at.slice(0, 16).replace("T", " ") }}</time>
          </div>
          <div class="comment-input">
            <ElInput placeholder="请输入评论内容..." />
            <ElButton type="primary">发表评论</ElButton>
          </div>
        </article>

        <article class="content-card">
          <h3>操作记录</h3>
          <ElTimeline>
            <ElTimelineItem
              v-for="log in detail.activity_logs"
              :key="log.id"
              :timestamp="log.created_at.slice(0, 16).replace('T', ' ')"
            >
              <strong>{{ log.action }}</strong>
              <p>{{ log.actor_name }}</p>
            </ElTimelineItem>
          </ElTimeline>
        </article>
      </aside>
    </div>

    <div class="bottom-actions">
      <ElButton type="primary"><ElIcon><Refresh /></ElIcon>更新状态</ElButton>
      <ElButton type="primary" plain><ElIcon><UploadFilled /></ElIcon>上传附件</ElButton>
      <ElButton type="primary" plain>发表评论</ElButton>
    </div>
  </div>
</template>
