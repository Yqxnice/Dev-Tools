<script setup lang="ts">
import { NTag, NIcon } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import {
  LayersOutline,
  LogoGithub,
  OpenOutline
} from '@vicons/ionicons5'

const PROJECT_REPO_URL = 'https://github.com/Yqxnice/Dev-Tools'
const techStack = ['Tauri 2', 'Vue 3', 'Rust', 'Naive UI']

function openProjectRepo() {
  open(PROJECT_REPO_URL)
}
</script>

<template>
  <div class="feature-panel about-page">
    <div class="feature-header">
      <div>
        <h3>关于</h3>
        <p>了解 Dev Tools 项目</p>
      </div>
    </div>

    <div class="about-body">
      <!-- 项目概览 -->
      <section class="about-card">
        <div class="about-header">
          <div class="about-logo">
            <n-icon :component="LayersOutline" />
          </div>
          <div class="about-title">
            <h2>Dev Tools</h2>
            <n-tag size="small" round>v0.1.0</n-tag>
          </div>
        </div>

        <p class="about-desc">
          Windows 本地开发环境管理面板，基于 Tauri 2 + Vue 3 + Rust 构建。
          提供 MySQL、PostgreSQL、Python、Java、Node.js、JetBrains 等运行时的
          版本检测、服务管理、智能卸载、残留清理、密码重置与镜像源切换等一站式能力。
        </p>

        <div class="about-tech">
          <n-tag v-for="t in techStack" :key="t" size="small" round :bordered="false">
            {{ t }}
          </n-tag>
        </div>
      </section>

      <!-- 项目地址 -->
      <section class="about-card">
        <h3 class="section-title">
          <n-icon :component="LogoGithub" />
          <span>项目地址</span>
        </h3>
        <div class="about-repo" title="点击在浏览器中打开" @click="openProjectRepo">
          <n-icon :component="LogoGithub" class="repo-icon" />
          <div class="repo-info">
            <span class="repo-label">GitHub 开源仓库</span>
            <span class="repo-url">{{ PROJECT_REPO_URL }}</span>
          </div>
          <n-icon :component="OpenOutline" class="repo-arrow" />
        </div>
        <p class="about-tip">点击仓库卡片即可在浏览器中访问项目主页</p>
      </section>
    </div>
  </div>
</template>

<style scoped>
.about-page { padding: 20px 24px; }
.about-body {
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-width: 860px;
}

/* 卡片（与设置页卡片风格一致） */
.about-card {
  background: var(--bg-card);
  border: 1px solid var(--border-primary);
  border-radius: 12px;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
}
.section-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 12px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border-primary);
}
.section-title .n-icon { font-size: 16px; color: var(--color-primary); }

/* 概览 */
.about-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 14px;
}
.about-logo {
  width: 42px;
  height: 42px;
  flex-shrink: 0;
  border-radius: 11px;
  background: linear-gradient(135deg, var(--color-primary), var(--color-primary-hover));
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-on-primary);
}
.about-logo .n-icon { font-size: 24px; }
.about-title {
  display: flex;
  align-items: center;
  gap: 8px;
}
.about-title h2 {
  margin: 0;
  font-size: 18px;
  font-weight: 700;
  letter-spacing: -0.01em;
  color: var(--text-primary);
}
.about-desc {
  margin: 0 0 14px;
  font-size: 13px;
  line-height: 1.7;
  color: var(--text-secondary);
}
.about-tech {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

/* 仓库卡片 */
.about-repo {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  border: 1px solid var(--border-primary);
  border-radius: 10px;
  background: var(--bg-secondary);
  cursor: pointer;
  transition: border-color 0.15s ease, background 0.15s ease;
}
.about-repo:hover {
  border-color: var(--color-primary);
  background: var(--color-primary-light);
}
.repo-icon { font-size: 22px; color: var(--text-primary); flex-shrink: 0; }
.repo-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.repo-label { font-size: 12px; font-weight: 600; color: var(--text-primary); }
.repo-url {
  font-size: 12px;
  color: var(--color-primary);
  word-break: break-all;
  font-family: 'JetBrains Mono', Consolas, monospace;
}
.repo-arrow { font-size: 15px; color: var(--text-muted); flex-shrink: 0; }
.about-tip {
  margin: 8px 0 0;
  font-size: 11px;
  color: var(--text-muted);
}
</style>
