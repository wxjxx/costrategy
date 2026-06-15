<script setup lang="ts">
import { InfoFilled } from "@element-plus/icons-vue";
import { useRouter } from "vue-router";
import { resetAuthenticationState } from "@/auth/sessionState";
import unauthorizedImage from "@/assets/401.png";

const router = useRouter();

function goHome() {
  resetAuthenticationState();
  void router.push("/");
}
</script>

<template>
  <main class="unauthorized-page">
    <section class="unauthorized-content" aria-labelledby="unauthorized-title">
      <img
        class="unauthorized-visual"
        :src="unauthorizedImage"
        alt="401 无权限访问插画"
      />

      <h1 id="unauthorized-title">抱歉，您无权访问此页面</h1>
      <p class="unauthorized-message">
        您当前没有权限访问该资源，请联系系统管理员获取访问权限。
      </p>
      <p class="unauthorized-code">
        <ElIcon><InfoFilled /></ElIcon>
        错误代码： 401 Unauthorized
      </p>
      <ElButton class="home-button" type="primary" @click="goHome">
        返回首页
      </ElButton>
    </section>
  </main>
</template>

<style scoped>
.unauthorized-page {
  min-height: 100vh;
  display: grid;
  place-items: center;
  padding: 12px 20px 24px;
  background:
    radial-gradient(circle at 50% 38%, rgba(71, 145, 255, 0.08), transparent 28%),
    linear-gradient(180deg, #fbfdff 0%, #f7faff 100%);
  color: #19365c;
}

.unauthorized-content {
  width: min(760px, 100%);
  text-align: center;
}

.unauthorized-visual {
  display: block;
  width: min(640px, 100%);
  max-height: 380px;
  object-fit: contain;
  margin: 0 auto 12px;
}

.unauthorized-content h1 {
  margin: 0;
  font-size: 30px;
  line-height: 1.35;
  font-weight: 800;
  color: #19365c;
}

.unauthorized-message {
  margin: 8px 0 18px;
  font-size: 18px;
  color: #6b7c99;
}

.unauthorized-code {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 0 0 20px;
  padding-top: 18px;
  width: 520px;
  max-width: 100%;
  margin-right: auto;
  margin-left: auto;
  justify-content: center;
  border-top: 1px solid #eef3fb;
  color: #1f3d66;
  font-size: 16px;
}

.unauthorized-code .el-icon {
  color: #63a0ff;
}

.home-button {
  width: 230px;
  height: 52px;
  border-radius: 8px;
  font-size: 18px;
  font-weight: 700;
  box-shadow: 0 12px 24px rgba(15, 107, 255, 0.24);
}

@media (max-width: 760px) {
  .unauthorized-visual {
    width: min(520px, 100%);
    margin-bottom: 10px;
  }

  .unauthorized-content h1 {
    font-size: 24px;
  }

  .unauthorized-message {
    font-size: 15px;
  }

  .unauthorized-code {
    min-width: 0;
    width: 100%;
    margin-bottom: 32px;
  }
}
</style>
