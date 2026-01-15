<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useI18n } from 'vue-i18n';
import ImmersiveCard from './ImmersiveCard.vue';
import ImmersiveSidebar from './ImmersiveSidebar.vue';
import Toast from './Toast.vue';

const { t } = useI18n();

// Types
interface FavoriteEntry {
  command: string;
  description: string;
}

interface Favorites {
  version: number;
  updated_at: number;
  items: { [key: string]: FavoriteEntry[] };
}

interface ParsedFavorite {
  scope: 'command' | 'shortcut';
  pageTitle: string;
  command: string;
  description: string;
  anchorId: string;
}

interface AppSettings {
  color: string;
  hotkey_toggle: string;
  always_on_top: boolean;
  theme: string;
  immersive_on_open_action: string;
  immersive_always_on_top: boolean;
  immersive_opacity: number;
  immersive_default_mode: string;
  immersive_sidebar_default: boolean;
}

// Filter state
type FilterMode = 'all' | 'command' | 'shortcut';
const filterMode = ref<FilterMode>('all');
const searchQuery = ref('');
const debouncedSearchQuery = ref('');

// Toast state
const toastVisible = ref(false);
const toastMessage = ref('');
const toastType = ref<'success' | 'error'>('success');

// Sidebar state
const sidebarVisible = ref(false);
const activeCardId = ref<string>('');
const contentRef = ref<HTMLElement | null>(null);
const commandsExpanded = ref(true);
const shortcutsExpanded = ref(true);
const immersiveOpacity = ref(1);
const searchPlaceholder = computed(() => {
  const value = t('search.searchPlaceholder');
  if (value && value !== 'search.searchPlaceholder') {
    return value;
  }
  const fallback = t('search.commandPlaceholder');
  if (fallback && fallback !== 'search.commandPlaceholder') {
    return fallback;
  }
  return 'Search...';
});

// Favorites data
const favorites = ref<ParsedFavorite[]>([]);
const loading = ref(true);

// Pagination state
const ITEMS_PER_PAGE = 40;
const currentPage = ref(1);

// Debounce search query
let debounceTimer: number | null = null;
watch(searchQuery, (newVal) => {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }
  
  debounceTimer = window.setTimeout(() => {
    debouncedSearchQuery.value = newVal;
    currentPage.value = 1; // Reset to first page on search
  }, 200);
});

// Track scroll position for sidebar highlighting
let scrollThrottle: number | null = null;
function handleScroll() {
  if (scrollThrottle) return;
  
  scrollThrottle = window.setTimeout(() => {
    scrollThrottle = null;
    updateActiveCard();
  }, 150);
}

// Update active card based on scroll position
function updateActiveCard() {
  const container = contentRef.value;
  if (!container) {
    return;
  }

  const cards = Array.from(container.querySelectorAll('[id^="card-"]')) as HTMLElement[];
  if (cards.length === 0) {
    return;
  }

  const containerRect = container.getBoundingClientRect();
  const centerY = containerRect.top + containerRect.height / 2;
  
  let closestCard: HTMLElement | null = null;
  let closestDistance = Infinity;
  
  for (const card of cards) {
    const rect = card.getBoundingClientRect();
    const cardCenterY = rect.top + rect.height / 2;
    const distance = Math.abs(cardCenterY - centerY);
    
    if (distance < closestDistance) {
      closestDistance = distance;
      closestCard = card;
    }
  }
  
  if (closestCard) {
    activeCardId.value = closestCard.id;
  }
}

// Load favorites
async function loadFavorites() {
  try {
    loading.value = true;
    const data = await invoke<Favorites>('get_favorites');
    favorites.value = parseFavorites(data);
  } catch (err) {
    console.error('Failed to load favorites:', err);
    favorites.value = [];
    // Show error toast
    toastMessage.value = t('common.loadFailed') || '加载失败，请重试';
    toastType.value = 'error';
    toastVisible.value = true;
  } finally {
    loading.value = false;
  }
}

// Parse favorites from backend format
function parseFavorites(data: Favorites): ParsedFavorite[] {
  const result: ParsedFavorite[] = [];
  
  for (const [key, entries] of Object.entries(data.items)) {
    // Parse scope::title format
    const parts = key.split('::');
    let scope: 'command' | 'shortcut' = 'command';
    let pageTitle = key;
    if (parts.length === 2) {
      const [rawScope, rawTitle] = parts;
      if (rawScope === 'command' || rawScope === 'shortcut') {
        scope = rawScope;
        pageTitle = rawTitle;
      }
    }
    
    // Add each entry
    entries.forEach((entry, idx) => {
      const anchorId = formatAnchorId(scope, pageTitle, idx);
      result.push({
        scope,
        pageTitle,
        command: entry.command,
        description: entry.description || '',
        anchorId
      });
    });
  }
  
  return result;
}

function formatAnchorId(scope: 'command' | 'shortcut', pageTitle: string, index: number): string {
  const normalized = pageTitle.trim().replace(/[\s/]+/g, '-');
  return `card-${scope}-${normalized || 'untitled'}-${index}`;
}

// Filtered items (with debounced search)
const filteredItems = computed(() => {
  let items = favorites.value;
  
  // Filter by mode
  if (filterMode.value === 'command') {
    items = items.filter(item => item.scope === 'command');
  } else if (filterMode.value === 'shortcut') {
    items = items.filter(item => item.scope === 'shortcut');
  }
  
  // Filter by debounced search query
  if (debouncedSearchQuery.value.trim()) {
    const query = debouncedSearchQuery.value.toLowerCase();
    items = items.filter(item => 
      item.pageTitle.toLowerCase().includes(query) ||
      item.command.toLowerCase().includes(query) ||
      item.description.toLowerCase().includes(query)
    );
  }
  
  return items;
});

const scopedItems = computed(() => {
  const commands = filteredItems.value.filter(item => item.scope === 'command');
  const shortcuts = filteredItems.value.filter(item => item.scope === 'shortcut');
  return { commands, shortcuts };
});

const groupedFavorites = computed(() => groupByTitle(scopedItems.value));

// Paginated items (for large datasets)
const paginatedItems = computed(() => {
  const grouped = scopedItems.value;
  const totalItems = grouped.commands.length + grouped.shortcuts.length;
  
  // Only paginate if more than 120 items
  if (totalItems <= 120) {
    return grouped;
  }
  
  const start = (currentPage.value - 1) * ITEMS_PER_PAGE;
  const end = start + ITEMS_PER_PAGE;
  
  const allItems = [...grouped.commands, ...grouped.shortcuts];
  const pageItems = allItems.slice(start, end);

  return {
    commands: pageItems.filter(item => item.scope === 'command'),
    shortcuts: pageItems.filter(item => item.scope === 'shortcut')
  };
});

// Total pages
const totalPages = computed(() => {
  const grouped = scopedItems.value;
  const totalItems = grouped.commands.length + grouped.shortcuts.length;
  return Math.ceil(totalItems / ITEMS_PER_PAGE);
});

// Show pagination
const showPagination = computed(() => {
  const grouped = scopedItems.value;
  const totalItems = grouped.commands.length + grouped.shortcuts.length;
  return totalItems > 120;
});

// Group by title for sidebar
function groupByTitle(items: { commands: ParsedFavorite[]; shortcuts: ParsedFavorite[] }) {
  const commands: { [key: string]: ParsedFavorite[] } = {};
  const shortcuts: { [key: string]: ParsedFavorite[] } = {};
  
  items.commands.forEach(item => {
    if (!commands[item.pageTitle]) {
      commands[item.pageTitle] = [];
    }
    commands[item.pageTitle].push(item);
  });

  items.shortcuts.forEach(item => {
    if (!shortcuts[item.pageTitle]) {
      shortcuts[item.pageTitle] = [];
    }
    shortcuts[item.pageTitle].push(item);
  });
  
  return { commands, shortcuts };
}

// Close window
async function closeWindow() {
  try {
    await invoke('close_immersive_window');
  } catch (err) {
    console.error('Failed to close immersive window:', err);
  }
}

// Copy to clipboard with toast
async function copyToClipboard(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    toastMessage.value = t('common.copied') || '已复制到剪贴板';
    toastType.value = 'success';
    toastVisible.value = true;
  } catch (err) {
    console.error('Failed to copy:', err);
    toastMessage.value = t('common.copyFailed') || '复制失败';
    toastType.value = 'error';
    toastVisible.value = true;
  }
}

// Pagination controls
function nextPage() {
  if (currentPage.value < totalPages.value) {
    currentPage.value++;
    contentRef.value?.scrollTo({ top: 0, behavior: 'smooth' });
  }
}

function prevPage() {
  if (currentPage.value > 1) {
    currentPage.value--;
    contentRef.value?.scrollTo({ top: 0, behavior: 'smooth' });
  }
}

// Load and apply settings
async function loadSettings() {
  try {
    const settings = await invoke<AppSettings>('get_app_settings');
    
    // Apply default filter mode
    if (settings.immersive_default_mode === 'command') {
      filterMode.value = 'command';
    } else if (settings.immersive_default_mode === 'shortcut') {
      filterMode.value = 'shortcut';
    } else {
      filterMode.value = 'all';
    }
    
    // Apply sidebar default visibility
    sidebarVisible.value = settings.immersive_sidebar_default;
    const rawOpacity = Number.isFinite(settings.immersive_opacity)
      ? settings.immersive_opacity
      : 1.0;
    immersiveOpacity.value = Math.min(1, Math.max(0.7, rawOpacity));
  } catch (err) {
    console.error('Failed to load settings:', err);
  }
}

// Listen for favorites updates
let unlistenFavorites: (() => void) | null = null;

onMounted(async () => {
  // Load settings first
  await loadSettings();
  
  // Then load favorites
  await loadFavorites();
  
  // Listen for favorites-updated event
  unlistenFavorites = await listen('favorites-updated', async () => {
    await loadFavorites();
  });
  
  // Add scroll listener
  contentRef.value?.addEventListener('scroll', handleScroll);
  
  // Initial active card update
  setTimeout(updateActiveCard, 100);
});

onUnmounted(() => {
  if (unlistenFavorites) {
    unlistenFavorites();
    unlistenFavorites = null;
  }
  contentRef.value?.removeEventListener('scroll', handleScroll);
  if (scrollThrottle) {
    clearTimeout(scrollThrottle);
  }
  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }
});
</script>

<template>
  <div class="immersive-view" :style="{ opacity: immersiveOpacity }">
    <!-- Search Bar -->
    <div class="search-bar" :class="{ 'sidebar-visible': sidebarVisible }">
      <!-- Exit Button -->
      <button class="exit-btn" @click="closeWindow">
        ← {{ t('common.backToMain') || '返回主窗口' }}
      </button>

      <!-- Filter Buttons -->
      <div class="filter-buttons">
        <button
          class="filter-btn"
          :class="{ active: filterMode === 'command', commands: true }"
          :title="t('search.commands')"
          @click="filterMode = 'command'"
        >
          <span class="dot"></span>
        </button>
        <button
          class="filter-btn"
          :class="{ active: filterMode === 'shortcut', shortcuts: true }"
          :title="t('search.shortcuts')"
          @click="filterMode = 'shortcut'"
        >
          <span class="dot"></span>
        </button>
        <button
          class="filter-btn"
          :class="{ active: filterMode === 'all', all: true }"
          :title="t('search.all')"
          @click="filterMode = 'all'"
        >
          <span class="dot"></span>
        </button>
      </div>
      
      <!-- Search Input -->
      <input
        v-model="searchQuery"
        type="text"
        class="search-input"
        :placeholder="searchPlaceholder"
      />
    </div>
    
    <!-- Sidebar -->
    <ImmersiveSidebar
      v-model:visible="sidebarVisible"
      :favorites="groupedFavorites"
      :active-card-id="activeCardId"
    />
    
    <!-- Content Area -->
    <div ref="contentRef" class="content-area" :class="{ 'sidebar-visible': sidebarVisible }">
      <!-- Loading State -->
      <div v-if="loading" class="loading-state">
        <div class="loading-spinner"></div>
        <p>{{ t('common.loading') || '加载中...' }}</p>
      </div>
      
      <!-- Empty State -->
      <div v-else-if="favorites.length === 0" class="empty-state">
        <div class="empty-icon">📭</div>
        <h3>{{ t('settings.noFavorites') || '暂无收藏' }}</h3>
        <p>{{ t('settings.noFavoritesHint') || '在主窗口中添加收藏后，即可在此查看' }}</p>
        <button class="empty-action-btn" @click="closeWindow">
          {{ t('common.backToMain') || '返回主窗口' }}
        </button>
      </div>
      
      <!-- No Results State -->
      <div v-else-if="scopedItems.commands.length === 0 && scopedItems.shortcuts.length === 0" class="empty-state">
        <div class="empty-icon">🔍</div>
        <h3>{{ t('search.noResults') || '未找到结果' }}</h3>
        <p>{{ t('search.tryDifferentKeywords') || '尝试使用不同的关键词' }}</p>
      </div>
      
      <!-- Favorites Grid -->
      <div v-else class="favorites-grid">
        <!-- Commands Section -->
        <div v-if="paginatedItems.commands.length > 0" class="scope-section">
          <h2 class="scope-header commands" @click="commandsExpanded = !commandsExpanded">
            <span class="scope-toggle">{{ commandsExpanded ? '▼' : '▶' }}</span>
            📦 {{ t('search.commands') }} ({{ scopedItems.commands.length }})
          </h2>
          <div v-show="commandsExpanded" class="cards-grid">
            <ImmersiveCard
              v-for="item in paginatedItems.commands"
              :key="item.anchorId"
              :page-title="item.pageTitle"
              :item="item"
              scope="command"
              @copy="copyToClipboard"
            />
          </div>
        </div>
        
        <!-- Shortcuts Section -->
        <div v-if="paginatedItems.shortcuts.length > 0" class="scope-section">
          <h2 class="scope-header shortcuts" @click="shortcutsExpanded = !shortcutsExpanded">
            <span class="scope-toggle">{{ shortcutsExpanded ? '▼' : '▶' }}</span>
            ⌨️ {{ t('search.shortcuts') }} ({{ scopedItems.shortcuts.length }})
          </h2>
          <div v-show="shortcutsExpanded" class="cards-grid">
            <ImmersiveCard
              v-for="item in paginatedItems.shortcuts"
              :key="item.anchorId"
              :page-title="item.pageTitle"
              :item="item"
              scope="shortcut"
              @copy="copyToClipboard"
            />
          </div>
        </div>
        
        <!-- Pagination -->
        <div v-if="showPagination" class="pagination">
          <button 
            class="pagination-btn" 
            :disabled="currentPage === 1"
            @click="prevPage"
          >
            ‹ {{ t('common.previous') || '上一页' }}
          </button>
          <span class="pagination-info">
            {{ currentPage }} / {{ totalPages }}
          </span>
          <button 
            class="pagination-btn" 
            :disabled="currentPage === totalPages"
            @click="nextPage"
          >
            {{ t('common.next') || '下一页' }} ›
          </button>
        </div>
      </div>
    </div>
    
    <!-- Toast Notification -->
    <Toast 
      v-model:visible="toastVisible" 
      :message="toastMessage"
      :type="toastType"
    />
  </div>
</template>

<style scoped>
.immersive-view {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #ffffff;
  overflow: hidden;
  --sidebar-width: 200px;
  --sidebar-gap: 8px;
}

/* Search Bar */
.search-bar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 20px;
  border-bottom: 1px solid #e1e4e8;
  background: #fafbfc;
  position: relative;
  z-index: 200;
}

.search-bar.sidebar-visible {
  margin-left: calc(var(--sidebar-width) + var(--sidebar-gap));
}

.exit-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border: 1px solid #d7dade;
  background: #ffffff;
  border-radius: 6px;
  font-size: 13px;
  color: #2a2a2a;
  cursor: pointer;
  transition: all 0.2s ease;
}

.exit-btn:hover {
  background: #f6f7f8;
  border-color: #8b95a0;
  transform: translateY(-1px);
}

.filter-buttons {
  display: flex;
  gap: 8px;
}

.filter-btn {
  width: 24px;
  height: 24px;
  border: 2px solid;
  border-radius: 50%;
  background: transparent;
  cursor: pointer;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
}

.filter-btn .dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  transition: all 0.2s ease;
}

.filter-btn.commands {
  border-color: #4f87ff;
}

.filter-btn.commands.active .dot {
  background: #4f87ff;
}

.filter-btn.shortcuts {
  border-color: #f7a34b;
}

.filter-btn.shortcuts.active .dot {
  background: #f7a34b;
}

.filter-btn.all {
  border-color: #8b95a0;
}

.filter-btn.all.active .dot {
  background: #8b95a0;
}

.filter-btn:hover {
  transform: scale(1.1);
}

.search-input {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid #d7dade;
  border-radius: 6px;
  font-size: 14px;
  outline: none;
  background: #fafbfc;
  color: #2a2a2a;
}

.search-input:focus {
  border-color: #4f87ff;
  box-shadow: 0 0 0 3px rgba(79, 135, 255, 0.1);
  background: #ffffff;
}

/* Content Area */
.content-area {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
  transition: margin-left 0.2s ease;
}

.content-area.sidebar-visible {
  margin-left: calc(var(--sidebar-width) + var(--sidebar-gap));
}

/* Loading State */
.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #586069;
}

.loading-spinner {
  width: 40px;
  height: 40px;
  border: 4px solid #f6f8fa;
  border-top-color: #4f87ff;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: 16px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Empty State */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #586069;
}

.empty-icon {
  font-size: 64px;
  margin-bottom: 16px;
}

.empty-state h3 {
  margin: 0 0 8px 0;
  font-size: 18px;
  color: #24292e;
}

.empty-state p {
  margin: 0 0 20px 0;
  font-size: 14px;
}

.empty-action-btn {
  padding: 10px 24px;
  border: 1px solid #d7dade;
  background: #ffffff;
  border-radius: 6px;
  font-size: 14px;
  color: #2a2a2a;
  cursor: pointer;
  transition: all 0.2s ease;
}

.empty-action-btn:hover {
  background: #f6f7f8;
  border-color: #8b95a0;
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.empty-action-btn:active {
  transform: translateY(0);
}

/* Favorites Grid */
.favorites-grid {
  max-width: 1400px;
  margin: 0 auto;
}

.scope-section {
  margin-bottom: 40px;
}

.scope-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 24px;
  font-weight: 600;
  margin: 0 0 16px 0;
  padding-bottom: 12px;
  border-bottom: 2px solid;
  cursor: pointer;
  user-select: none;
}

.scope-toggle {
  font-size: 14px;
  color: #6b6b6b;
}

.scope-header.commands {
  color: #4f87ff;
  border-bottom-color: #4f87ff;
}

.scope-header.shortcuts {
  color: #f7a34b;
  border-bottom-color: #f7a34b;
}

.cards-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 20px;
}

@media (max-width: 1200px) {
  .cards-grid {
    grid-template-columns: 1fr;
  }
}

/* Highlight Flash Animation */
:deep(.highlight-flash) {
  animation: highlight-pulse 1.5s ease-out;
}

@keyframes highlight-pulse {
  0% {
    box-shadow: 0 0 0 0 rgba(3, 102, 214, 0.7);
    transform: scale(1);
  }
  50% {
    box-shadow: 0 0 0 10px rgba(3, 102, 214, 0);
    transform: scale(1.02);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(3, 102, 214, 0);
    transform: scale(1);
  }
}

/* Pagination */
.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
  margin-top: 40px;
  padding: 20px 0;
}

.pagination-btn {
  padding: 8px 16px;
  border: 1px solid #d1d5da;
  background: #ffffff;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s ease;
  color: #24292e;
}

.pagination-btn:hover:not(:disabled) {
  background: #f6f8fa;
  border-color: #959da5;
}

.pagination-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.pagination-info {
  font-size: 14px;
  color: #586069;
  min-width: 80px;
  text-align: center;
}
</style>
