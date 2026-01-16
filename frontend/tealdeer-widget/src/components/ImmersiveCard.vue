<script setup lang="ts">
import { computed } from 'vue';

interface FavoriteItem {
  command: string;
  description: string;
  anchorId?: string;
}

interface Props {
  pageTitle: string;
  item: FavoriteItem;
  scope: 'command' | 'shortcut';
}

const props = defineProps<Props>();
const emit = defineEmits<{
  copy: [text: string];
}>();

function formatCardId(scope: 'command' | 'shortcut', pageTitle: string): string {
  const normalized = pageTitle.trim().replace(/[\s/]+/g, '-');
  return `card-${scope}-${normalized || 'untitled'}`;
}

const cardId = computed(() => {
  return props.item.anchorId || formatCardId(props.scope, props.pageTitle);
});

function handleCardClick() {
  emit('copy', props.item.command);
}
</script>

<template>
  <div
    :id="cardId"
    class="immersive-card"
    :class="scope"
    @click="handleCardClick"
  >
    <div class="card-header">
      <h3 class="card-title">{{ pageTitle }}</h3>
    </div>
    
    <div class="card-content">
      <p v-if="item.description" class="item-description">
        {{ item.description }}
      </p>
      <code class="item-command">{{ item.command }}</code>
    </div>
  </div>
</template>

<style scoped>
.immersive-card {
  background: var(--panel-bg);
  border-left: 4px solid;
  border-radius: 8px;
  padding: 16px;
  box-shadow: var(--panel-shadow);
  cursor: pointer;
  transition: all 0.3s ease;
}

.immersive-card:hover {
  transform: translateY(-4px);
  box-shadow: var(--panel-shadow);
  background: var(--bg-alt);
}

.immersive-card:active {
  transform: scale(0.98);
}

.immersive-card.command:hover {
  background: var(--bg-alt);
}

.immersive-card.shortcut:hover {
  background: var(--bg-alt);
}

.immersive-card.command {
  border-left-color: var(--command-accent);
}

.immersive-card.shortcut {
  border-left-color: var(--shortcut-accent);
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--panel-border);
}

.card-title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.card-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.item-description {
  margin: 0;
  font-size: 14px;
  color: var(--text-muted);
  line-height: 1.5;
}

.item-command {
  display: block;
  padding: 8px 12px;
  background: var(--code-inline-bg);
  border: 1px solid var(--panel-border);
  border-radius: 6px;
  font-size: 14px;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  color: var(--text-primary);
  word-break: break-all;
}
</style>
