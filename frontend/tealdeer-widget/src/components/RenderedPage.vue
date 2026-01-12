<template>
  <div class="rendered-page">
    <!-- 页面标题和全局操作 -->
    <div class="page-header-section">
      <h1>{{ parsedPage.title }}</h1>
      <div class="global-actions">
        <button class="action-btn" @click="$emit('copyAll')">
          📋 {{ t('search.copyAll') }}
        </button>
        <button class="action-btn" @click="$emit('favoriteAll')">
          ⭐ {{ t('search.favoriteAll') }}
        </button>
      </div>
    </div>
    
    <!-- 描述 -->
    <blockquote v-if="parsedPage.description" class="page-description">
      {{ parsedPage.description }}
    </blockquote>
    
    <!-- 示例列表 -->
    <div class="examples-list">
      <div v-for="(example, index) in parsedPage.examples" :key="index" class="example-item">
        <p v-if="example.description" class="example-desc">{{ example.description }}</p>
        <CommandLine
          :line-number="index + 1"
          :command="example.command"
          :is-favorited="isFavorited(example.command)"
          @copy="(cmd) => $emit('copy', cmd)"
          @favorite="() => $emit('favorite', { command: example.command, description: example.description })"
        />
      </div>
    </div>

    <div
      v-if="extrasHtml"
      class="page-extras"
      v-html="extrasHtml"
    ></div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import CommandLine from './CommandLine.vue';
import type { ParsedPage } from '../utils/tldr_parser';

const { t } = useI18n();

defineProps<{
  parsedPage: ParsedPage;
  isFavorited: (command: string) => boolean;
  extrasHtml?: string;
}>();

defineEmits<{
  copy: [command: string];
  favorite: [payload: { command: string; description: string }];
  copyAll: [];
  favoriteAll: [];
}>();
</script>

<style scoped>
.rendered-page {
  padding: 16px 0;
}

.page-header-section {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  flex-wrap: wrap;
  gap: 12px;
}

.page-header-section h1 {
  margin: 0;
  font-size: 2rem;
  color: var(--text-primary);
}

.global-actions {
  display: flex;
  gap: 12px;
}

.action-btn {
  padding: 8px 16px;
  background: var(--button-ghost-bg);
  border: 1px solid var(--button-ghost-border);
  border-radius: 8px;
  cursor: pointer;
  font-size: 0.9rem;
  font-weight: 600;
  transition: all 0.2s ease;
  white-space: nowrap;
}

.action-btn:hover {
  background: var(--accent-outline);
  transform: translateY(-2px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.page-description {
  margin: 16px 0;
  padding: 12px 16px;
  background: var(--panel-bg);
  border-left: 4px solid var(--accent);
  border-radius: 8px;
  color: var(--text-muted-strong);
  font-style: italic;
}

.examples-list {
  margin-top: 24px;
}

.example-item {
  margin-bottom: 16px;
}

.example-desc {
  margin: 0 0 8px 0;
  color: var(--text-primary);
  font-weight: 500;
}

.page-extras :deep(p) {
  margin: 12px 0 0;
  color: var(--text-muted-strong);
}

.page-extras :deep(a) {
  color: var(--accent);
  text-decoration: none;
}

.page-extras :deep(a:hover) {
  text-decoration: underline;
}
</style>
