<template>
  <div class="command-line">
    <span class="line-number">{{ lineNumber }}</span>
    <code class="command-code">
      <span
        v-for="(token, idx) in tokens"
        :key="idx"
        :class="`token-${token.type}`"
      >{{ token.value }}</span>
    </code>
    <div class="command-actions">
      <button
        class="action-btn copy-btn"
        :title="t('search.copy')"
        @click="$emit('copy', command)"
      >
        📋
      </button>
      <button
        class="action-btn favorite-btn"
        :class="{ favorited: isFavorited }"
        :disabled="isFavorited"
        :title="isFavorited ? t('settings.favoriteExists') : t('settings.favoriteAdded')"
        @click="$emit('favorite', command)"
      >
        {{ isFavorited ? '⭐' : '☆' }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { tokenizeCommand } from '../utils/command_tokens';

const { t } = useI18n();

const props = defineProps<{
  lineNumber: number;
  command: string;
  isFavorited: boolean;
}>();

defineEmits<{
  copy: [command: string];
  favorite: [command: string];
}>();

const tokens = computed(() => tokenizeCommand(props.command));
</script>

<style scoped>
.command-line {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  background: var(--code-block-bg);
  border-radius: 8px;
  margin: 8px 0;
  border: 1px solid var(--panel-border);
}

.line-number {
  min-width: 30px;
  text-align: right;
  color: var(--text-muted);
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.85rem;
  user-select: none;
}

.command-code {
  flex: 1;
  overflow-x: auto;
  background: transparent !important;
  padding: 0 !important;
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.9rem;
  color: var(--code-block-text);
}

/* Syntax highlighting tokens */
.token-command {
  color: #0066cc;
  font-weight: 600;
}

.token-sudo {
  color: #cc0000;
  font-weight: 600;
}

.token-option {
  color: #0099cc;
}

.token-variable {
  color: #cc6600;
  font-style: italic;
}

.token-string {
  color: #009900;
}

.token-operator {
  color: #666666;
  font-weight: 600;
}

.token-path {
  color: #9966cc;
}

.token-text {
  color: var(--code-block-text);
}

/* Dark theme adjustments */
:root[data-theme='dark'] .token-command {
  color: #66b3ff;
}

:root[data-theme='dark'] .token-sudo {
  color: #ff6666;
}

:root[data-theme='dark'] .token-option {
  color: #66ccff;
}

:root[data-theme='dark'] .token-variable {
  color: #ffaa66;
}

:root[data-theme='dark'] .token-string {
  color: #66ff66;
}

:root[data-theme='dark'] .token-operator {
  color: #aaaaaa;
}

:root[data-theme='dark'] .token-path {
  color: #cc99ff;
}

.command-actions {
  display: flex;
  gap: 8px;
}

.action-btn {
  padding: 6px 12px;
  background: var(--button-ghost-bg);
  border: 1px solid var(--button-ghost-border);
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.9rem;
  transition: all 0.2s ease;
  white-space: nowrap;
}

.action-btn:hover:not(:disabled) {
  background: var(--accent-outline);
  transform: translateY(-2px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.favorite-btn.favorited {
  background: var(--accent-outline);
  color: var(--accent);
  border-color: var(--accent);
  cursor: not-allowed;
  opacity: 0.6;
}

.action-btn:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}
</style>
