<template>
  <div class="favorites-panel">
    <div class="panel-header">
      <h3>{{ t('settings.myFavorites') }}</h3>
      <button 
        v-if="hasFavorites"
        class="clear-all-btn"
        @click="$emit('clearAll')"
      >
        {{ t('settings.clearAll') }}
      </button>
    </div>

    <div v-if="!hasFavorites" class="empty-state">
      {{ t('settings.noFavorites') }}
    </div>

    <div v-else class="favorites-list">
      <div 
        v-for="(entries, pageTitle) in favorites.items" 
        :key="pageTitle"
        class="favorite-group"
      >
        <div class="group-header" @click="toggleGroup(pageTitle)">
          <span class="toggle-icon">{{ isExpanded(pageTitle) ? '▼' : '▶' }}</span>
          <span class="group-title">{{ pageTitle }}</span>
          <span class="group-count">({{ entries.length }})</span>
        </div>
        
        <div v-show="isExpanded(pageTitle)" class="group-commands">
          <div 
            v-for="(entry, idx) in entries" 
            :key="`${entry.command}-${idx}`"
            class="command-item"
          >
            <div class="command-info">
              <p v-if="entry.description" class="command-desc">
                {{ entry.description }}
              </p>
              <code class="command-text">{{ entry.command }}</code>
            </div>
            <div class="command-actions">
              <button 
                class="action-btn copy-btn"
                :title="t('search.copy')"
                @click="$emit('copy', entry.command)"
              >
                📋
              </button>
              <button 
                class="action-btn remove-btn"
                :title="t('settings.remove')"
                @click="$emit('remove', pageTitle, entry.command)"
              >
                🗑️
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

interface FavoriteEntry {
  command: string;
  description: string;
}

interface Favorites {
  version: number;
  updated_at: number;
  items: Record<string, FavoriteEntry[]>;
}

const props = defineProps<{
  favorites: Favorites;
}>();

defineEmits<{
  copy: [command: string];
  remove: [pageTitle: string, command: string];
  clearAll: [];
}>();

const expandedGroups = ref<Set<string>>(new Set());

const hasFavorites = computed(() => {
  return Object.keys(props.favorites.items).length > 0;
});

function toggleGroup(pageTitle: string) {
  if (expandedGroups.value.has(pageTitle)) {
    expandedGroups.value.delete(pageTitle);
  } else {
    expandedGroups.value.add(pageTitle);
  }
}

function isExpanded(pageTitle: string): boolean {
  return expandedGroups.value.has(pageTitle);
}

// Auto-expand first group on mount
onMounted(() => {
  if (Object.keys(props.favorites.items).length > 0) {
    const firstGroup = Object.keys(props.favorites.items)[0];
    expandedGroups.value.add(firstGroup);
  }
});
</script>

<style scoped>
.favorites-panel {
  margin-top: 24px;
  padding: 16px;
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  border-radius: 12px;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.panel-header h3 {
  margin: 0;
  font-size: 1.2rem;
  color: var(--text-primary);
}

.clear-all-btn {
  padding: 6px 12px;
  background: var(--alert-bg);
  border: 1px solid var(--alert-border);
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.85rem;
  color: var(--alert-text);
  transition: all 0.2s ease;
}

.clear-all-btn:hover {
  background: var(--alert-border);
}

.empty-state {
  padding: 32px;
  text-align: center;
  color: var(--text-muted);
  font-style: italic;
}

.favorites-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.favorite-group {
  border: 1px solid var(--panel-border);
  border-radius: 8px;
  overflow: hidden;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  background: var(--code-block-bg);
  cursor: pointer;
  user-select: none;
  transition: background 0.2s ease;
}

.group-header:hover {
  background: var(--accent-outline);
}

.toggle-icon {
  font-size: 0.8rem;
  color: var(--text-muted);
}

.group-title {
  flex: 1;
  font-weight: 600;
  color: var(--text-primary);
  font-family: 'JetBrains Mono', monospace;
}

.group-count {
  color: var(--text-muted);
  font-size: 0.9rem;
}

.group-commands {
  padding: 8px;
  background: var(--panel-bg);
}

.command-item {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px;
  margin: 4px 0;
  background: var(--code-block-bg);
  border: 1px solid var(--panel-border);
  border-radius: 6px;
  transition: all 0.2s ease;
}

.command-item:hover {
  border-color: var(--accent);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.command-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.command-desc {
  margin: 0;
  color: var(--code-block-text);
  font-size: 0.85rem;
  opacity: 0.8;
}

.command-text {
  overflow-x: auto;
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.9rem;
  color: var(--code-block-text);
  background: transparent !important;
  padding: 0 !important;
}

.command-actions {
  display: flex;
  gap: 6px;
}

.action-btn {
  padding: 4px 10px;
  background: var(--button-ghost-bg);
  border: 1px solid var(--button-ghost-border);
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.85rem;
  transition: all 0.2s ease;
}

.action-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
}

.copy-btn:hover {
  background: var(--accent-outline);
  border-color: var(--accent);
}

.remove-btn:hover {
  background: var(--alert-bg);
  border-color: var(--alert-border);
}
/* Light theme: light purple background, dark blue text */
:root[data-theme="light"] .command-item {
  background: #e9e0ff;
}

:root[data-theme="light"] .command-text {
  color: #1e3a8a;
}
:root[data-theme="light"] .command-desc {
  color: #4a5568;
}

/* Dark theme: keep current dark background, white text */
:root[data-theme="dark"] .command-text {
  color: #ffffff;
}
</style>
