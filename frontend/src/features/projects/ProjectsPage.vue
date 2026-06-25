<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useMutation, useQuery, useQueryClient } from "@tanstack/vue-query";
import { Plus, Search } from "@element-plus/icons-vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { api } from "@/api/client";
import UserAvatar from "@/components/UserAvatar.vue";
import { selectableUsers } from "@/features/users/userOptions";
import type { CreateProjectPayload, Project, ProjectStatus, UpdateProjectPayload } from "@/types";

const queryClient = useQueryClient();
const { data: projects } = useQuery({ queryKey: ["projects"], queryFn: api.projects });
const { data: users } = useQuery({ queryKey: ["users"], queryFn: api.users });

const keyword = ref("");
const status = ref<ProjectStatus | "">("");
const ownerId = ref("");
const currentPage = ref(1);
const pageSize = ref(10);
const dialogVisible = ref(false);
const editingProjectId = ref("");
const projectForm = reactive<CreateProjectPayload>({
  name: "",
  owner_id: undefined,
  description: "",
  start_date: "",
  end_date: "",
  status: "active",
});

const filteredProjects = computed(() =>
  (projects.value ?? []).filter((project) => {
    if (keyword.value && !project.name.includes(keyword.value)) return false;
    if (status.value && project.status !== status.value) return false;
    if (ownerId.value && project.owner_id !== ownerId.value) return false;
    return true;
  }),
);
const projectOwnerOptions = computed(() => selectableUsers(users.value ?? []));
const pagedProjects = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value;
  return filteredProjects.value.slice(start, start + pageSize.value);
});

watch([keyword, status, ownerId], () => {
  currentPage.value = 1;
});

watch(filteredProjects, (items) => {
  const maxPage = Math.max(1, Math.ceil(items.length / pageSize.value));
  if (currentPage.value > maxPage) currentPage.value = maxPage;
});

function ownerName(ownerIdValue?: string): string {
  return users.value?.find((user) => user.id === ownerIdValue)?.name ?? "-";
}

function statusText(value: ProjectStatus): string {
  return { active: "进行中", archived: "已归档", completed: "已完成", paused: "已暂停" }[value];
}

function resetProjectFilters() {
  keyword.value = "";
  status.value = "";
  ownerId.value = "";
  currentPage.value = 1;
}

function runProjectSearch() {
  currentPage.value = 1;
}

const saveMutation = useMutation({
  mutationFn: () => {
    if (editingProjectId.value) {
      const payload: UpdateProjectPayload = {
        name: projectForm.name,
        owner_id: projectForm.owner_id,
        description: projectForm.description,
        start_date: projectForm.start_date || undefined,
        end_date: projectForm.end_date || undefined,
        status: projectForm.status,
      };
      return api.updateProject(editingProjectId.value, payload);
    }
    return api.createProject({
      ...projectForm,
      start_date: projectForm.start_date || undefined,
      end_date: projectForm.end_date || undefined,
    });
  },
  onSuccess: () => {
    ElMessage.success("项目已保存");
    dialogVisible.value = false;
  },
  onSettled: () => queryClient.invalidateQueries({ queryKey: ["projects"] }),
});

const deleteMutation = useMutation({
  mutationFn: (projectId: string) => api.deleteProject(projectId),
  onSuccess: () => ElMessage.success("项目已删除"),
  onError: () => ElMessage.error("只有系统管理员可以删除项目"),
  onSettled: () => queryClient.invalidateQueries({ queryKey: ["projects"] }),
});

async function deleteProject(project: Project) {
  try {
    await ElMessageBox.confirm(`确认删除项目“${project.name}”？`, "删除项目", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
    });
    deleteMutation.mutate(project.id);
  } catch {
    // User cancelled.
  }
}

function openProjectDialog(project?: Project) {
  editingProjectId.value = project?.id ?? "";
  projectForm.name = project?.name ?? "";
  projectForm.owner_id = project?.owner_id;
  projectForm.description = project?.description ?? "";
  projectForm.start_date = project?.start_date ?? "";
  projectForm.end_date = project?.end_date ?? "";
  projectForm.status = project?.status ?? "active";
  dialogVisible.value = true;
}
</script>

<template>
  <div class="projects-page">
    <section class="content-card search-panel">
      <ElForm label-position="top">
        <ElRow :gutter="40">
          <ElCol :xs="24" :sm="12" :md="6">
            <ElFormItem label="项目名称关键词">
              <ElInput v-model="keyword" placeholder="请输入项目名称关键词">
                <template #suffix><ElIcon><Search /></ElIcon></template>
              </ElInput>
            </ElFormItem>
          </ElCol>
          <ElCol :xs="24" :sm="12" :md="6">
            <ElFormItem label="项目状态">
              <ElSelect v-model="status" clearable placeholder="请选择项目状态">
                <ElOption label="进行中" value="active" />
                <ElOption label="已完成" value="completed" />
                <ElOption label="已暂停" value="paused" />
                <ElOption label="已归档" value="archived" />
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :xs="24" :sm="12" :md="6">
            <ElFormItem label="项目负责人">
              <ElSelect v-model="ownerId" clearable placeholder="请选择项目负责人">
                <ElOption
                  v-for="user in projectOwnerOptions"
                  :key="user.id"
                  :label="user.name"
                  :value="user.id"
                />
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :xs="24" :sm="12" :md="6" class="search-actions">
            <ElButton @click="resetProjectFilters">重置</ElButton>
            <ElButton type="primary" @click="runProjectSearch">搜索</ElButton>
          </ElCol>
        </ElRow>
      </ElForm>
    </section>

    <section class="content-card">
      <div class="section-heading">
        <ElButton type="primary" @click="openProjectDialog()">
          <ElIcon><Plus /></ElIcon>
          新建项目
        </ElButton>
      </div>
      <ElTable :data="pagedProjects" class="project-table">
        <ElTableColumn label="项目名称" min-width="260">
          <template #default="{ row }">
            <strong>{{ row.name }}</strong>
          </template>
        </ElTableColumn>
        <ElTableColumn label="项目负责人" width="180">
          <template #default="{ row }">
            <span class="table-user"><UserAvatar :name="ownerName(row.owner_id)" />{{ ownerName(row.owner_id) }}</span>
          </template>
        </ElTableColumn>
        <ElTableColumn label="项目描述" prop="description" min-width="320" />
        <ElTableColumn prop="start_date" label="开始日期" width="130" />
        <ElTableColumn prop="end_date" label="结束日期" width="130" />
        <ElTableColumn label="项目状态" width="130">
          <template #default="{ row }">
            <ElTag :type="row.status === 'active' ? 'success' : row.status === 'completed' ? 'primary' : 'warning'">
              {{ statusText(row.status) }}
            </ElTag>
          </template>
        </ElTableColumn>
        <ElTableColumn label="操作" width="160">
          <template #default="{ row }">
            <ElButton link type="primary" @click="openProjectDialog(row)">编辑</ElButton>
            <ElButton link type="danger" @click="deleteProject(row)">删除</ElButton>
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
          :total="filteredProjects.length"
        />
      </div>
    </section>

    <ElDialog v-model="dialogVisible" :title="editingProjectId ? '编辑项目' : '新建项目'" width="600" class="form-dialog">
      <ElForm label-width="98px">
        <ElFormItem label="项目名称" required><ElInput v-model="projectForm.name" /></ElFormItem>
        <ElFormItem label="项目负责人" required>
          <ElSelect v-model="projectForm.owner_id" clearable>
            <ElOption
              v-for="user in projectOwnerOptions"
              :key="user.id"
              :label="user.name"
              :value="user.id"
            />
          </ElSelect>
        </ElFormItem>
        <ElFormItem label="项目描述" required>
          <ElInput v-model="projectForm.description" type="textarea" :rows="4" />
        </ElFormItem>
        <ElFormItem label="开始日期">
          <ElDatePicker v-model="projectForm.start_date" value-format="YYYY-MM-DD" />
        </ElFormItem>
        <ElFormItem label="结束日期">
          <ElDatePicker v-model="projectForm.end_date" value-format="YYYY-MM-DD" />
        </ElFormItem>
        <ElFormItem label="项目状态" required>
          <ElSelect v-model="projectForm.status">
            <ElOption label="进行中" value="active" />
            <ElOption label="已完成" value="completed" />
            <ElOption label="已暂停" value="paused" />
            <ElOption label="已归档" value="archived" />
          </ElSelect>
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="dialogVisible = false">取消</ElButton>
        <ElButton type="primary" :loading="saveMutation.isPending.value" @click="saveMutation.mutate()">保存</ElButton>
      </template>
    </ElDialog>
  </div>
</template>
