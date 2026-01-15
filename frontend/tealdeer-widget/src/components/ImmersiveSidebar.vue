<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';

interface FavoriteItem {
  command: string;
  description: string;
  anchorId?: string;
}

interface Props {
  visible: boolean;
  favorites: {
    commands: { [key: string]: FavoriteItem[] };
    shortcuts: { [key: string]: FavoriteItem[] };
  };
  activeCardId?: string;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  'update:visible': [value: boolean];
}>();

const autoVisible = ref(props.visible);
const commandsExpanded = ref(true);
const shortcutsExpanded = ref(true);

// Throttle mouse move handler
let throttleTimer: number | null = null;
function handleMouseMove(e: MouseEvent) {
  if (throttleTimer) return;
  
  throttleTimer = window.setTimeout(() => {
    throttleTimer = null;
    
    // Show sidebar when mouse is near left edge (within 10px)
    if (e.clientX < 10) {
      autoVisible.value = true;
    } else if (e.clientX > 220) {
      autoVisible.value = false;
    }
  }, 100);
}

// Computed visibility
const isVisible = computed(() => autoVisible.value);

// Scroll to card with highlight
function scrollToCard(cardId: string) {
  const element = document.getElementById(cardId);
  if (element) {
    element.scrollIntoView({
      behavior: 'smooth',
      block: 'center'
    });
    
    // Add highlight animation
    element.classList.add('highlight-flash');
    setTimeout(() => {
      element.classList.remove('highlight-flash');
    }, 1500);
  }
}

// Check if item is active
function isActive(entries: FavoriteItem[]): boolean {
  return entries.some((entry) => entry.anchorId === props.activeCardId);
}

function resolveAnchor(entries: FavoriteItem[]): string | null {
  return entries[0]?.anchorId ?? null;
}

function scrollToFirst(entries: FavoriteItem[]) {
  const anchor = resolveAnchor(entries);
  if (anchor) {
    scrollToCard(anchor);
  }
}

onMounted(() => {
  window.addEventListener('mousemove', handleMouseMove);
});

onUnmounted(() => {
  window.removeEventListener('mousemove', handleMouseMove);
  if (throttleTimer) {
    clearTimeout(throttleTimer);
  }
});

watch(
  () => props.visible,
  (value) => {
    if (autoVisible.value !== value) {
      autoVisible.value = value;
    }
  }
);

watch(autoVisible, (value) => {
  emit('update:visible', value);
});
</script>

<template>
  <div class="immersive-sidebar" :class="{ visible: isVisible }">
    <div class="sidebar-header">
      <h4>📑 {{ $t('common.catalog') || '目录' }}</h4>
    </div>
    
    <div class="sidebar-content">
      <!-- Commands Group -->
      <div v-if="Object.keys(favorites.commands).length > 0" class="sidebar-group">
        <div 
          class="group-header commands" 
          @click="commandsExpanded = !commandsExpanded"
        >
          <span class="expand-icon">{{ commandsExpanded ? '▼' : '▶' }}</span>
          📦 {{ $t('search.commands') }} ({{ Object.keys(favorites.commands).length }})
        </div>
        <div v-show="commandsExpanded" class="group-items">
          <div
            v-for="pageTitle in Object.keys(favorites.commands)"
            :key="`nav-command-${pageTitle}`"
            class="group-item"
            :class="{ active: isActive(favorites.commands[pageTitle]) }"
            @click="scrollToFirst(favorites.commands[pageTitle])"
          >
            {{ pageTitle }}
          </div>
        </div>
      </div>
      
      <!-- Shortcuts Group -->
      <div v-if="Object.keys(favorites.shortcuts).length > 0" class="sidebar-group">
        <div 
          class="group-header shortcuts" 
          @click="shortcutsExpanded = !shortcutsExpanded"
        >
          <span class="expand-icon">{{ shortcutsExpanded ? '▼' : '▶' }}</span>
          ⌨️ {{ $t('search.shortcuts') }} ({{ Object.keys(favorites.shortcuts).length }})
        </div>
        <div v-show="shortcutsExpanded" class="group-items">
          <div
            v-for="pageTitle in Object.keys(favorites.shortcuts)"
            :key="`nav-shortcut-${pageTitle}`"
            class="group-item"
            :class="{ active: isActive(favorites.shortcuts[pageTitle]) }"
            @click="scrollToFirst(favorites.shortcuts[pageTitle])"
          >
            {{ pageTitle }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.immersive-sidebar {
  position: fixed;
  left: -200px;
  top: 0;
  width: 200px;
  height: 100vh;
  background: #fafbfc;
  border-right: 1px solid #e1e4e8;
  display: flex;
  flex-direction: column;
  transition: left 0.2s ease;
  z-index: 100;
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.05);
}

.immersive-sidebar.visible {
  left: 0;
}

.sidebar-header {
  padding: 16px;
  border-bottom: 1px solid #e1e4e8;
}

.sidebar-header h4 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: #24292e;
}

.sidebar-content {
  flex: 1;
  overflow-y: auto;
  padding: 12px 0;
}

.sidebar-group {
  margin-bottom: 16px;
}

.group-header {
  padding: 8px 16px;
  font-size: 14px;
  font-weight: 600;
  color: #24292e;
  cursor: pointer;
  user-select: none;
  display: flex;
  align-items: center;
  gap: 6px;
  transition: background 0.2s ease;
}

.group-header:hover {
  background: #f6f8fa;
}

.expand-icon {
  font-size: 10px;
  color: #586069;
  transition: transform 0.2s ease;
}

.group-header.commands {
  color: #4f87ff;
}

.group-header.shortcuts {
  color: #f7a34b;
}

.group-items {
  display: flex;
  flex-direction: column;
}

.group-item {
  padding: 8px 16px 8px 32px;
  font-size: 14px;
  color: #586069;
  cursor: pointer;
  transition: all 0.2s ease;
  position: relative;
}

.group-item::before {
  content: '';
  position: absolute;
  left: 16px;
  top: 50%;
  transform: translateY(-50%);
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: #d1d5da;
  transition: all 0.2s ease;
}

.group-item:hover {
  background: #f6f8fa;
  color: #24292e;
}

.group-item:hover::before {
  width: 6px;
  height: 6px;
  background: #586069;
}

.group-item.active {
  background: #e8f4fd;
  color: #0366d6;
  font-weight: 500;
}

.group-item.active::before {
  width: 6px;
  height: 6px;
  background: #0366d6;
}
</style>
