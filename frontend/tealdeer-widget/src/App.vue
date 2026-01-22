<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { join } from "@tauri-apps/api/path";
import { error as logError, info as logInfo, warn as logWarn } from "@tauri-apps/plugin-log";
import MarkdownIt from "markdown-it";
import { useI18n } from "vue-i18n";
import { saveLocale } from "./locales";
import logoLight from "../static/logo/logo_light.svg";
import sunIcon from "../static/ico/sun.svg";
import moonIcon from "../static/ico/moon.svg";
import RenderedPage from "./components/RenderedPage.vue";
import FavoritesPanel from "./components/FavoritesPanel.vue";
import {
  parseTldrMarkdown,
  normalizeCommand,
  type ParsedPage,
} from "./utils/tldr_parser";

const { t, locale } = useI18n();

type RenderResult = {
  stdout: string;
  stderr: string;
  status: number | null;
  timed_out: boolean;
};

type CustomFileInfo = {
  path: string;
  slug: string;
  bytes: number;
};

type CustomEntry = {
  command_slug: string;
  display_command: string;
  kind: "page" | "patch";
  status: "enabled" | "disabled";
  path: string;
  mtime: number;
  size: number;
  summary: string | null;
  examples_count: number;
  pageType?: "command" | "shortcut";
};

type SearchEntry = {
  name: string;
  summary: string | null;
  scope: "command" | "shortcut";
  source: "tldr" | "custom" | "shortcut";
};

type ShowPaths = {
  config_dir: string | null;
  config_path: string | null;
  cache_dir: string | null;
  pages_dir: string | null;
  custom_pages_dir: string | null;
  shortcut_pages_dir: string | null;
};

type ThemeMode = "light" | "dark";

type AppSettings = {
  color: string;
  hotkey_toggle: string;
  always_on_top: boolean;
  theme: ThemeMode;
  immersive_on_open_action: string;
  immersive_always_on_top: boolean;
  immersive_opacity: number;
  immersive_default_mode: string;
  immersive_sidebar_default: boolean;
};

type TealdeerConfigValues = {
  languages: string[];
  platforms: string[];
  auto_update: boolean;
  auto_update_interval_hours: number | null;
  use_pager: boolean;
  archive_source: string | null;
};

type TealdeerConfigPatch = {
  languages?: string[] | null;
  platforms?: string[] | null;
  auto_update?: boolean;
  auto_update_interval_hours?: number | null;
  use_pager?: boolean;
  archive_source?: string | null;
};

type ExampleInput = {
  desc: string;
  cmd: string;
};

type NewPageRequest = {
  command: string;
  summary: string;
  examples: ExampleInput[];
};

type NewPatchRequest = {
  command: string;
  examples: ExampleInput[];
  include_header_in_patch: boolean;
};

type FavoriteEntry = {
  command: string;
  description: string;
};

type Favorites = {
  version: number;
  updated_at: number;
  items: Record<string, FavoriteEntry[]>;
};

const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: false,
});

const activeTab = ref<"search" | "new" | "manage" | "settings">("search");
const pageScope = ref<"all" | "command" | "shortcut">("all");

const commandInput = ref("");
const language = ref("");
const platforms = ref<string[]>(["linux", "common"]);
const rawOutput = ref("");
const isNoResults = ref(false);
const isLoading = ref(false);
const isOffline = ref(!navigator.onLine);
const showOfflineBanner = ref(false);
let offlineBannerTimer: number | null = null;
let offlineBannerInterval: number | null = null;
const viewMode = ref<"rendered" | "raw">("rendered");
const lastCommand = ref("");
const updateProgress = ref("");
const lastRenderScope = ref<"command" | "shortcut">("command");
let themeTransitionTimer: number | null = null;

const searchResults = ref<SearchEntry[]>([]);
const searchResultsQuery = ref("");

// 搜索历史和自动补全
const searchHistory = ref<string[]>([]);
const availableCommands = ref<string[]>([]);
const showSuggestions = ref(false);
const suggestions = ref<string[]>([]);
const selectedSuggestionIndex = ref(-1);
const commandInputRef = ref<HTMLInputElement | null>(null);

// Toast 提示
const toastMessage = ref("");
const showToast = ref(false);
const toastType = ref<"success" | "error">("success");

// 收藏功能
const favorites = ref<Favorites>({ version: 2, updated_at: 0, items: {} });
const favoriteIndex = ref<Map<string, Set<string>>>(new Map());

// 临时 placeholder 状态
const commandPlaceholderTemp = ref('');
const commandPlaceholderTimer = ref<number | null>(null);
const newCommandPlaceholderTemp = ref('');
const newCommandPlaceholderTimer = ref<number | null>(null);

const newMode = ref<"page" | "patch" | "append">("page");
const newPageType = ref<"command" | "shortcut">("command");
const newCommand = ref("");
const summary = ref("");
const includePatchHeader = ref(false);
const examples = ref<ExampleInput[]>([{ desc: "", cmd: "" }]);
const isCreating = ref(false);

const manageEntries = ref<CustomEntry[]>([]);
const manageFilterType = ref<"all" | "command" | "shortcut">("all");
const manageLoading = ref(false);
const manageType = ref<"all" | "page" | "patch">("all");
const manageStatus = ref<"all" | "enabled" | "disabled">("all");
const manageQuery = ref("");

const settingsLoading = ref(false);
const settingsLanguages = ref("");
const settingsPlatforms = ref<string[]>(["linux", "common"]);
const settingsDisableAutoUpdate = ref(false);
const settingsInterval = ref(24);
const settingsUsePager = ref(false);
const settingsArchiveSource = ref("");
const settingsColor = ref("auto");
const settingsHotkey = ref("Ctrl+Alt+T");
const settingsAlwaysOnTop = ref(true);
const settingsImmersiveOnOpen = ref("hide_to_tray");
const settingsImmersiveAlwaysOnTop = ref(true);
const settingsImmersiveOpacity = ref(1.0);
const settingsImmersiveDefaultMode = ref("all");
const settingsImmersiveSidebarDefault = ref(true);
const theme = ref<ThemeMode>("light");
const showPaths = ref<ShowPaths | null>(null);
const logDir = ref<string | null>(null);
const logRustPath = ref<string | null>(null);
const logWebviewPath = ref<string | null>(null);
let unlistenNavigate: (() => void) | null = null;
let windowErrorHandler: ((event: ErrorEvent) => void) | null = null;
let windowRejectionHandler: ((event: PromiseRejectionEvent) => void) | null = null;
let handleOnline: (() => void) | null = null;
let handleOffline: (() => void) | null = null;

const platformOptions = [
  { value: "linux", label: "Linux" },
  { value: "common", label: "Common" },
  { value: "macos", label: "macOS" },
  { value: "windows", label: "Windows" },
];

// const invalidReason = computed(() => validateCommandString(commandInput.value));
// const previewInvalidReason = computed(() => validateCommandString(newCommand.value));
// const newInvalidReason = computed(() => {
//   const commandReason = validateCommandString(newCommand.value);
//   if (commandReason) {
//     return commandReason;
//   }

//   if (newMode.value === "page" && !summary.value.trim()) {
//     return t("validation.summaryRequired");
//   }

//   const neededExamples = newMode.value === "append" ? 1 : examples.value.length;
//   if (neededExamples === 0) {
//     return t("validation.exampleRequired");
//   }

//   for (let i = 0; i < neededExamples; i += 1) {
//     const example = examples.value[i];
//     if (!example || !example.desc.trim() || !example.cmd.trim()) {
//       return t("validation.exampleComplete");
//     }
//   }

//   return "";
// });

const filteredEntries = computed(() => {
  const query = manageQuery.value.trim().toLowerCase();
  return manageEntries.value.filter((entry) => {
    // Filter by page type (command/shortcut)
    if (manageFilterType.value !== "all") {
      if (entry.pageType !== manageFilterType.value) return false;
    }
    
    if (manageType.value !== "all" && entry.kind !== manageType.value) {
      return false;
    }
    if (manageStatus.value !== "all" && entry.status !== manageStatus.value) {
      return false;
    }
    if (!query) {
      return true;
    }
    const summary = entry.summary ? entry.summary.toLowerCase() : "";
    return (
      entry.command_slug.toLowerCase().includes(query) ||
      entry.display_command.toLowerCase().includes(query) ||
      summary.includes(query)
    );
  });
});

const themeIcon = computed(() =>
  theme.value === "light" ? moonIcon : sunIcon,
);
const themeToggleTitle = computed(() =>
  theme.value === "light" ? "Switch to dark theme" : "Switch to light theme",
);
const logoSrc = computed(() => logoLight);

const parsedPage = computed<ParsedPage | null>(() => {
  if (!rawOutput.value) {
    return null;
  }
  try {
    const parsed = parseTldrMarkdown(rawOutput.value);
    if (!parsed) {
      return null;
    }
    if (!parsed.examples.length && !parsed.description && !parsed.extras.length) {
      return null;
    }
    return parsed;
  } catch (err) {
    logUiError(`Failed to parse TLDR: ${normalizeError(err)}`);
    return null;
  }
});

const extrasHtml = computed(() => {
  if (!parsedPage.value?.extras.length) {
    return "";
  }
  return md.render(parsedPage.value.extras.join("\n"));
});

const renderedHtml = computed(() => {
  if (!rawOutput.value) {
    return "";
  }
  let html = md.render(rawOutput.value);
  
  // 高亮搜索的命令
  if (lastCommand.value) {
    const commandRegex = new RegExp(`\\b${lastCommand.value}\\b`, 'gi');
    html = html.replace(commandRegex, '<mark>$&</mark>');
  }
  
  return html;
});

const groupedSearchResults = computed(() => {
  return {
    command: searchResults.value.filter((entry) => entry.scope === "command"),
    shortcut: searchResults.value.filter((entry) => entry.scope === "shortcut"),
  };
});

function tokenizeCommand(value: string): string[] {
  return value.trim().split(/\s+/).filter(Boolean);
}

// 计算属性：动态 placeholder
const commandPlaceholder = computed(() => {
  return commandPlaceholderTemp.value || t('search.commandPlaceholder');
});

const newCommandPlaceholder = computed(() => {
  return newCommandPlaceholderTemp.value || t('newPage.commandPlaceholder');
});

// 显示 "Command is required" 提示
function showCommandRequiredHint(
  inputValue: string,
  placeholderRef: typeof commandPlaceholderTemp,
  timerRef: typeof commandPlaceholderTimer
): boolean {
  if (!inputValue.trim()) {
    // 清除之前的定时器
    if (timerRef.value) {
      clearTimeout(timerRef.value);
    }
    
    // 设置临时 placeholder
    placeholderRef.value = t('validation.commandRequired');
    
    // 60 秒后恢复
    timerRef.value = window.setTimeout(() => {
      placeholderRef.value = '';
      timerRef.value = null;
    }, 60000);
    
    return true; // 输入无效
  }
  return false; // 输入有效
}

// function validateCommandString(value: string): string {
//   const trimmed = value.trim();
//   if (!trimmed) {
//     return t("validation.commandRequired");
//   }
//   if (trimmed.includes("/") || trimmed.includes("\\") || trimmed.includes("..")) {
//     return t("validation.commandInvalidPath");
//   }
//   return "";
// }

function normalizeError(err: unknown): string {
  if (typeof err === "string") {
    return err;
  }
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: string }).message);
  }
  try {
    return JSON.stringify(err);
  } catch {
    return "Unknown error.";
  }
}

function formatSearchSource(source: string): string {
  if (source === "tldr") return t("search.sourceTldr");
  if (source === "custom") return t("search.sourceCustom");
  if (source === "shortcut") return t("search.sourceShortcut");
  return source;
}

function logUiInfo(message: string) {
  void logInfo(`[ui] ${message}`).catch(() => {});
}

function logUiWarn(message: string) {
  void logWarn(`[ui] ${message}`).catch(() => {});
}

function logUiError(message: string) {
  void logError(`[ui] ${message}`).catch(() => {});
}

function normalizeTheme(value: string | null | undefined): ThemeMode {
  return value === "dark" ? "dark" : "light";
}

function applyTheme(next: ThemeMode) {
  theme.value = next;
  const root = document.documentElement;
  root.classList.add("theme-transition");
  if (themeTransitionTimer) {
    clearTimeout(themeTransitionTimer);
  }
  themeTransitionTimer = window.setTimeout(() => {
    root.classList.remove("theme-transition");
    themeTransitionTimer = null;
  }, 320);
  root.dataset.theme = next;
}

function buildAppSettingsPayload(nextTheme?: ThemeMode): AppSettings {
  return {
    color: settingsColor.value || "auto",
    hotkey_toggle: settingsHotkey.value.trim(),
    always_on_top: settingsAlwaysOnTop.value,
    theme: nextTheme ?? theme.value,
    immersive_on_open_action: settingsImmersiveOnOpen.value,
    immersive_always_on_top: settingsImmersiveAlwaysOnTop.value,
    immersive_opacity: settingsImmersiveOpacity.value,
    immersive_default_mode: settingsImmersiveDefaultMode.value,
    immersive_sidebar_default: settingsImmersiveSidebarDefault.value,
  };
}

async function toggleTheme() {
  const previous = theme.value;
  const next = previous === "light" ? "dark" : "light";
  applyTheme(next);
  try {
    const payload = buildAppSettingsPayload(next);
    await invoke("set_app_settings", { settings: payload });
    logUiInfo(`Theme set to ${next}`);
  } catch (err) {
    applyTheme(previous);
    logUiError(`Failed to save theme: ${normalizeError(err)}`);
  }
}

async function openImmersiveWindow() {
  try {
    await invoke('open_immersive_window');
    logUiInfo('Immersive window opened');
  } catch (err) {
    console.error('Failed to open immersive window:', err);
  }
}

function cleanExamples(): ExampleInput[] {
  return examples.value.map((example) => ({
    desc: example.desc.trim(),
    cmd: example.cmd.trim(),
  }));
}

function setSuccessMessage(message: string) {
  showSuccessToast(message);
}

function setErrorMessage(message: string) {
  showErrorToast(message);
}

function parseCsv(value: string): string[] {
  return value
    .split(/[,]+/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function formatTimestamp(seconds: number): string {
  if (!seconds) {
    return "Unknown";
  }
  return new Date(seconds * 1000).toLocaleString();
}

async function resolveLogPaths() {
  try {
    const dir = await invoke<string>("get_log_dir");
    logDir.value = dir;
    logRustPath.value = await join(dir, "rust.log");
    logWebviewPath.value = await join(dir, "webview.log");
  } catch (err) {
    logUiWarn(`Failed to resolve log paths: ${normalizeError(err)}`);
  }
}

async function loadManageEntries() {
  manageLoading.value = true;
  try {
    const [customPages, shortcutPages] = await Promise.all([
      invoke<CustomEntry[]>("scan_custom_pages"),
      invoke<CustomEntry[]>("scan_shortcut_pages")
    ]);
    
    // Mark entries with their source type
    const markedCustom = customPages.map(e => ({ ...e, pageType: "command" as const }));
    const markedShortcut = shortcutPages.map(e => ({ ...e, pageType: "shortcut" as const }));
    
    manageEntries.value = [...markedCustom, ...markedShortcut];
  } catch (err) {
    showErrorToast(normalizeError(err));
    logUiError(`Failed to scan pages: ${normalizeError(err)}`);
  } finally {
    manageLoading.value = false;
  }
}

async function toggleEntry(entry: CustomEntry) {
  manageLoading.value = true;
  try {
    const isShortcut = entry.pageType === "shortcut";
    if (entry.status === "enabled") {
      const cmd = isShortcut ? "disable_shortcut_file" : "disable_custom_file";
      await invoke<CustomFileInfo>(cmd, { path: entry.path });
    } else {
      const cmd = isShortcut ? "enable_shortcut_file" : "enable_custom_file";
      await invoke<CustomFileInfo>(cmd, { path: entry.path });
    }
    await loadManageEntries();
    showSuccessToast(entry.status === "enabled" ? "Disabled" : "Enabled");
  } catch (err) {
    showErrorToast(normalizeError(err));
    logUiError(`Failed to update custom entry: ${normalizeError(err)}`);
  } finally {
    manageLoading.value = false;
  }
}

async function deleteEntry(entry: CustomEntry) {
  const confirmed = window.confirm(
    `Delete ${entry.command_slug}? This cannot be undone.`,
  );
  if (!confirmed) {
    return;
  }
  manageLoading.value = true;
  try {
    const cmd = entry.pageType === "shortcut" ? "delete_shortcut_file" : "delete_custom_file";
    await invoke(cmd, { path: entry.path });
    await loadManageEntries();
    showSuccessToast("Deleted successfully");
  } catch (err) {
    showErrorToast(normalizeError(err));
    logUiError(`Failed to delete custom entry: ${normalizeError(err)}`);
  } finally {
    manageLoading.value = false;
  }
}

async function openEntry(entry: CustomEntry) {
  try {
    const cmd = entry.pageType === "shortcut" ? "open_shortcut_page_file" : "open_custom_page_file";
    await invoke(cmd, { filePath: entry.path });
  } catch (err) {
    showErrorToast(normalizeError(err));
    logUiError(`Failed to open custom entry: ${normalizeError(err)}`);
  }
}

async function loadSettings() {
  settingsLoading.value = true;
  // cleared;
  try {
    const [appSettings, configValues, paths] = await Promise.all([
      invoke<AppSettings>("get_app_settings"),
      invoke<TealdeerConfigValues>("get_tealdeer_config_values"),
      invoke<ShowPaths>("get_show_paths"),
    ]);

    settingsColor.value = appSettings.color || "auto";
    settingsHotkey.value = appSettings.hotkey_toggle;
    settingsAlwaysOnTop.value = appSettings.always_on_top;
    applyTheme(normalizeTheme(appSettings.theme));
    settingsImmersiveOnOpen.value = appSettings.immersive_on_open_action;
    settingsImmersiveAlwaysOnTop.value = appSettings.immersive_always_on_top;
    settingsImmersiveOpacity.value = appSettings.immersive_opacity;
    settingsImmersiveDefaultMode.value = appSettings.immersive_default_mode;
    settingsImmersiveSidebarDefault.value = appSettings.immersive_sidebar_default;

    settingsLanguages.value = configValues.languages.join(", ");
    settingsPlatforms.value =
      configValues.platforms.length > 0
        ? configValues.platforms
        : ["linux", "common"];
    settingsDisableAutoUpdate.value = !configValues.auto_update;
    settingsInterval.value = configValues.auto_update_interval_hours ?? 24;
    settingsUsePager.value = configValues.use_pager;
    settingsArchiveSource.value = configValues.archive_source ?? "";
    showPaths.value = paths;
  } catch (err) {
    showErrorToast(normalizeError(err));
    logUiError(`Failed to load settings: ${normalizeError(err)}`);
  } finally {
    settingsLoading.value = false;
  }
}

async function loadAppSettings() {
  try {
    const appSettings = await invoke<AppSettings>("get_app_settings");
    settingsColor.value = appSettings.color || "auto";
    settingsHotkey.value = appSettings.hotkey_toggle;
    settingsAlwaysOnTop.value = appSettings.always_on_top;
    applyTheme(normalizeTheme(appSettings.theme));
    settingsImmersiveOnOpen.value = appSettings.immersive_on_open_action;
    settingsImmersiveAlwaysOnTop.value = appSettings.immersive_always_on_top;
    settingsImmersiveOpacity.value = appSettings.immersive_opacity;
    settingsImmersiveDefaultMode.value = appSettings.immersive_default_mode;
    settingsImmersiveSidebarDefault.value = appSettings.immersive_sidebar_default;
  } catch (err) {
    showErrorToast(normalizeError(err));
    logUiError(`Failed to load app settings: ${normalizeError(err)}`);
  }
}

async function saveSettings() {
  settingsLoading.value = true;

  try {
    const appSettings = buildAppSettingsPayload();
    await invoke("set_app_settings", { settings: appSettings });

    const languages = parseCsv(settingsLanguages.value);
    const platforms =
      settingsPlatforms.value.length > 0 ? settingsPlatforms.value : null;
    const interval = Number.isFinite(settingsInterval.value)
      ? Math.max(1, settingsInterval.value)
      : null;
    const patch: TealdeerConfigPatch = {
      languages: languages.length > 0 ? languages : null,
      platforms,
      auto_update: !settingsDisableAutoUpdate.value,
      auto_update_interval_hours: interval,
      use_pager: settingsUsePager.value,
      archive_source: settingsArchiveSource.value.trim()
        ? settingsArchiveSource.value.trim()
        : null,
    };
    await invoke("set_tealdeer_config", { patch });
    
    // Reload paths after saving settings
    try {
      showPaths.value = await invoke<ShowPaths>("get_show_paths");
    } catch (pathErr) {
      logUiWarn(`Failed to reload paths: ${normalizeError(pathErr)}`);
    }
    
    setSuccessMessage("Settings saved.");
    logUiInfo("Settings saved");
  } catch (err) {
    setErrorMessage(normalizeError(err));
    logUiError(`Failed to save settings: ${normalizeError(err)}`);
  } finally {
    settingsLoading.value = false;
  }
}

async function openConfigFile() {
  try {
    await invoke("open_config_file");
  } catch (err) {
    showErrorToast(normalizeError(err));
    logUiError(`Failed to open config.toml: ${normalizeError(err)}`);
  }
}

async function openLogPath(path: string | null, label: string) {
  if (!path) {
    showErrorToast(`${label} is not available.`);
    return;
  }
  try {
    await invoke("open_log_directory", { logDir: path });
  } catch (err) {
    showErrorToast(normalizeError(err));
    logUiError(`Failed to open ${label}: ${normalizeError(err)}`);
  }
}

async function openLogDir() {
  await openLogPath(logDir.value, "Log directory");
}

async function openRustLog() {
  await openLogPath(logRustPath.value, "rust.log");
}

async function openWebviewLog() {
  await openLogPath(logWebviewPath.value, "webview.log");
}

async function openCustomDir() {
  try {
    const commandName =
      newPageType.value === "shortcut"
        ? "open_shortcut_pages_dir"
        : "open_custom_pages_dir";
    await invoke(commandName);
    logUiInfo("Opened pages directory");
  } catch (err) {
    setErrorMessage(normalizeError(err));
    logUiError(`Failed to open pages directory: ${normalizeError(err)}`);
  }
}

async function renderCommandWithScope(command: string, scope: "command" | "shortcut") {
  isLoading.value = true;
  rawOutput.value = "";
  isNoResults.value = false;

  try {
    const tokens = tokenizeCommand(command);
    logUiInfo(`Render run (${scope}): ${command}`);
    const result = await invoke<RenderResult>("render_tldr", {
      commandTokens: tokens,
      scope,
      language: language.value || null,
      platforms: platforms.value,
      raw: true,
      color: settingsColor.value || null,
      pager: false,
      noAutoUpdate: false,
    });
    rawOutput.value = result.stdout;
    lastCommand.value = command;
    lastRenderScope.value = scope;
    viewMode.value = "rendered";
  } catch (err) {
    const errorMsg = normalizeError(err);
    
    // 精确区分错误类型，避免误导用户
    if (errorMsg.includes("Page cache not found")) {
      showErrorToast(t("search.cacheMissing"));
      logUiError(`Cache not initialized: ${command}`);
    } else if (errorMsg.includes("not found in cache")) {
      rawOutput.value =
        scope === "command"
          ? t("search.noResultsCommand")
          : t("search.noResultsShortcut");
      viewMode.value = "rendered";
      isNoResults.value = true;
      logUiInfo(`Page not found in ${scope} scope: ${command}`);
    } else {
      showErrorToast(errorMsg);
      logUiError(`Render failed: ${errorMsg}`);
    }
  } finally {
    isLoading.value = false;
  }
}

async function runSearch() {
  // 验证输入
  if (showCommandRequiredHint(
    commandInput.value,
    commandPlaceholderTemp,
    commandPlaceholderTimer
  )) {
    return; // 输入无效，不执行搜索
  }
  
  // 验证路径分隔符
  const trimmed = commandInput.value.trim();
  if (trimmed.includes("/") || trimmed.includes("\\") || trimmed.includes("..")) {
    showErrorToast(t('validation.commandInvalidPath'));
    return;
  }

  try {
    const command = commandInput.value.trim();
    logUiInfo(`Search run: ${command}`);
    isNoResults.value = false;
    
    if (pageScope.value === "all") {
      isLoading.value = true;
      rawOutput.value = "";
      searchResults.value = [];
      searchResultsQuery.value = command;
      const result = await invoke<SearchEntry[]>("search_pages", {
        query: command,
        scope: "all",
      });
      searchResults.value = result;

      // All模式：如果没有结果，显示友好提示
      if (result.length === 0) {
        rawOutput.value = t("search.noResultsAll");
        viewMode.value = "rendered";
        isNoResults.value = true;
      }
    } else if (pageScope.value === "shortcut") {
      isLoading.value = true;
      rawOutput.value = "";
      searchResults.value = [];
      searchResultsQuery.value = "";

      const result = await invoke<SearchEntry[]>("search_pages", {
        query: command,
        scope: "shortcut",
      });

      const normalized = tokenizeCommand(command).join("-").toLowerCase();
      const exact = result.find(
        (entry) => entry.name.toLowerCase() === normalized,
      );

      if (!exact) {
        rawOutput.value = t("search.noResultsShortcut");
        viewMode.value = "rendered";
        isNoResults.value = true;
      } else {
        await renderCommandWithScope(exact.name, "shortcut");
      }
    } else {
      searchResults.value = [];
      searchResultsQuery.value = "";
      await renderCommandWithScope(command, "command");
    }

    addToSearchHistory(command);
    showSuggestions.value = false;
  } catch (err) {
    showErrorToast(normalizeError(err));
    logUiError(`Search failed: ${normalizeError(err)}`);
  } finally {
    if (pageScope.value !== "command") {
      isLoading.value = false;
    }
  }
}

async function selectSearchResult(entry: SearchEntry) {
  commandInput.value = entry.name;
  await renderCommandWithScope(entry.name, entry.scope);
}

async function updateCache() {
  if (isOffline.value) {
    showErrorToast(t("common.offlineMode"));
    return;
  }
  isLoading.value = true;
  updateProgress.value = "Starting update...";

  try {
    logUiInfo("Starting cache update");
    const result = await invoke<RenderResult>("update_cache");
    
    if (result.status === 0) {
      logUiInfo("Cache updated successfully");
      updateProgress.value = "";
      showSuccessToast(t('search.cacheUpdateSuccess'));
    } else {
      updateProgress.value = "";
      showErrorToast(result.stderr || "Update failed");
      logUiError(`Cache update failed: ${result.stderr}`);
    }
  } catch (err) {
    updateProgress.value = "";
    showErrorToast(`Update failed: ${normalizeError(err)}`);
    logUiError(`Cache update error: ${normalizeError(err)}`);
  } finally {
    isLoading.value = false;
  }
}

async function runPreview() {
  // 验证输入
  if (showCommandRequiredHint(
    newCommand.value,
    newCommandPlaceholderTemp,
    newCommandPlaceholderTimer
  )) {
    return; // 输入无效，不执行预览
  }
  
  // 验证路径分隔符
  const trimmed = newCommand.value.trim();
  if (trimmed.includes("/") || trimmed.includes("\\") || trimmed.includes("..")) {
    showErrorToast(t('validation.commandInvalidPath'));
    return;
  }

  try {
    const command = newCommand.value.trim();
    logUiInfo(`Preview run: ${command}`);
    await renderCommandWithScope(command, newPageType.value);
  } catch (err) {
    showErrorToast(normalizeError(err));
    logUiError(`Preview failed: ${normalizeError(err)}`);
  }
}

async function createCustom() {
  // 验证命令输入
  if (showCommandRequiredHint(
    newCommand.value,
    newCommandPlaceholderTemp,
    newCommandPlaceholderTimer
  )) {
    return;
  }
  
  // 验证路径分隔符
  const trimmed = newCommand.value.trim();
  if (trimmed.includes("/") || trimmed.includes("\\") || trimmed.includes("..")) {
    setErrorMessage(t('validation.commandInvalidPath'));
    return;
  }
  
  // 验证其他字段
  if (newMode.value === "page" && !summary.value.trim()) {
    setErrorMessage(t('validation.summaryRequired'));
    return;
  }
  
  const neededExamples = newMode.value === "append" ? 1 : examples.value.length;
  if (neededExamples === 0) {
    setErrorMessage(t('validation.exampleRequired'));
    return;
  }
  
  for (let i = 0; i < neededExamples; i += 1) {
    const example = examples.value[i];
    if (!example || !example.desc.trim() || !example.cmd.trim()) {
      setErrorMessage(t('validation.exampleComplete'));
      return;
    }
  }

  isCreating.value = true;

  try {
    const command = newCommand.value.trim();
    if (newMode.value === "page") {
      const req: NewPageRequest = {
        command,
        summary: summary.value.trim(),
        examples: cleanExamples(),
      };
      const commandName = newPageType.value === "shortcut" 
        ? "create_or_overwrite_shortcut_page" 
        : "create_or_overwrite_page";
      const result = await invoke<CustomFileInfo>(commandName, {
        req,
      });
      setSuccessMessage(`Saved page to ${result.path}`);
      logUiInfo(`Created custom page: ${command}`);
    } else if (newMode.value === "patch") {
      const req: NewPatchRequest = {
        command,
        examples: cleanExamples(),
        include_header_in_patch: includePatchHeader.value,
      };
      const commandName = newPageType.value === "shortcut"
        ? "create_or_overwrite_shortcut_patch"
        : "create_or_overwrite_patch";
      const result = await invoke<CustomFileInfo>(commandName, {
        req,
      });
      
      // Check for duplicate warning in result
      if (result.path.includes('|DUPCOUNT:')) {
        const [actualPath, countStr] = result.path.split('|DUPCOUNT:');
        const count = parseInt(countStr, 10);
        setSuccessMessage(`Appended patch to ${actualPath}`);
        // Show warning toast after a short delay
        setTimeout(() => {
          showErrorToast(t('newPage.duplicateWarning', { count }));
        }, 500);
        logUiInfo(`Appended custom patch with warning: ${command}`);
      } else {
        setSuccessMessage(`Saved patch to ${result.path}`);
        logUiInfo(`Created custom patch: ${command}`);
      }
    } else {
      const first = cleanExamples()[0];
      const commandName = newPageType.value === "shortcut"
        ? "append_example_to_shortcut_page"
        : "append_example_to_page";
      const result = await invoke<CustomFileInfo>(commandName, {
        command,
        example: first,
      });
      setSuccessMessage(`Appended example in ${result.path}`);
      logUiInfo(`Appended example: ${command}`);
    }
  } catch (err) {
    setErrorMessage(normalizeError(err));
    logUiError(`Failed to create custom content: ${normalizeError(err)}`);
  } finally {
    isCreating.value = false;
  }
}

// Toast 提示
function showToastMessage(message: string, type: "success" | "error" = "success") {
  toastMessage.value = message;
  toastType.value = type;
  showToast.value = true;
  setTimeout(() => {
    showToast.value = false;
  }, 5000);
}

function showSuccessToast(message: string) {
  showToastMessage(message, "success");
}

function showErrorToast(message: string) {
  showToastMessage(message, "error");
}

// 显示离线横幅（5秒后自动隐藏）
function displayOfflineBanner() {
  showOfflineBanner.value = true;
  if (offlineBannerTimer) {
    clearTimeout(offlineBannerTimer);
  }
  offlineBannerTimer = window.setTimeout(() => {
    showOfflineBanner.value = false;
  }, 5000);
}

// 启动离线横幅定时器（每2分钟显示一次）
function startOfflineBannerInterval() {
  if (offlineBannerInterval) {
    clearInterval(offlineBannerInterval);
  }
  displayOfflineBanner(); // 立即显示一次
  offlineBannerInterval = window.setInterval(() => {
    if (isOffline.value) {
      displayOfflineBanner();
    }
  }, 120000); // 2分钟 = 120000ms
}

// 停止离线横幅定时器
function stopOfflineBannerInterval() {
  if (offlineBannerInterval) {
    clearInterval(offlineBannerInterval);
    offlineBannerInterval = null;
  }
  if (offlineBannerTimer) {
    clearTimeout(offlineBannerTimer);
    offlineBannerTimer = null;
  }
  showOfflineBanner.value = false;
}


// Helper function to build favorite key with scope
function buildFavoriteKey(pageTitle: string, scope?: string): string {
  // If pageTitle already has scope prefix, return as is
  if (pageTitle.includes('::')) {
    return pageTitle;
  }
  
  // Determine scope: use provided scope, or infer from pageScope/newPageType
  const inferredScope =
    activeTab.value === 'search'
      ? (pageScope.value === 'all' ? lastRenderScope.value : pageScope.value)
      : activeTab.value === 'new'
        ? newPageType.value
        : 'command';
  const currentScope = scope || inferredScope;
  const finalScope = currentScope === 'all' ? lastRenderScope.value : currentScope;
  
  return `${finalScope}::${pageTitle}`;
}

function buildFavoriteIndex() {
  const index = new Map<string, Set<string>>();
  for (const [pageTitle, commands] of Object.entries(favorites.value.items)) {
    const normalizedSet = new Set(
      commands.map((entry) => normalizeCommand(entry.command)),
    );
    index.set(pageTitle, normalizedSet);
  }
  favoriteIndex.value = index;
}

async function loadFavorites() {
  try {
    const result = await invoke<Favorites>('get_favorites');
    favorites.value = result;
    buildFavoriteIndex();
    logUiInfo('Favorites loaded');
  } catch (err) {
    logUiWarn(`Failed to load favorites: ${normalizeError(err)}`);
  }
}

function isFavorite(pageTitle: string, command: string, scope?: string): boolean {
  const favoriteKey = buildFavoriteKey(pageTitle, scope);
  const normalizedCmd = normalizeCommand(command);
  return favoriteIndex.value.get(favoriteKey)?.has(normalizedCmd) || false;
}

type FavoriteAddOptions = {
  silent?: boolean;
  skipExistsToast?: boolean;
  scope?: string;
};

async function addToFavorites(
  pageTitle: string,
  command: string,
  description: string,
  options: FavoriteAddOptions = {},
): Promise<boolean> {
  if (!pageTitle) {
    if (!options.silent) {
      showErrorToast('No command context');
    }
    return false;
  }
  
  // Build key with scope prefix
  const favoriteKey = buildFavoriteKey(pageTitle, options.scope);
  
  if (isFavorite(favoriteKey, command)) {
    if (!options.silent && !options.skipExistsToast) {
      showErrorToast(t('settings.favoriteExists'));
    }
    return false;
  }
  
  try {
    const result = await invoke<Favorites>('add_favorite', {
      pageTitle: favoriteKey,
      command,
      description
    });
    favorites.value = result;
    buildFavoriteIndex();
    if (!options.silent) {
      showSuccessToast(t('settings.favoriteAdded'));
    }
    return true;
  } catch (err) {
    if (!options.silent) {
      showErrorToast(normalizeError(err));
    }
    logUiError(`Failed to add favorite: ${normalizeError(err)}`);
    return false;
  }
}

async function removeFromFavorites(pageTitle: string, command: string) {
  try {
    const result = await invoke<Favorites>('remove_favorite', {
      pageTitle,
      command
    });
    favorites.value = result;
    buildFavoriteIndex();
    showSuccessToast(t('settings.favoriteRemoved'));
  } catch (err) {
    showErrorToast(normalizeError(err));
    logUiError(`Failed to remove favorite: ${normalizeError(err)}`);
  }
}

async function clearAllFavorites() {
  try {
    const result = await invoke<Favorites>('clear_favorites');
    favorites.value = result;
    buildFavoriteIndex();
    showSuccessToast('All favorites cleared');
  } catch (err) {
    showErrorToast(normalizeError(err));
    logUiError(`Failed to clear favorites: ${normalizeError(err)}`);
  }
}

// 单个命令复制/收藏处理
async function handleCopyCommand(command: string) {
  try {
    await navigator.clipboard.writeText(command);
    showSuccessToast(t('search.copied'));
  } catch (err) {
    showErrorToast(normalizeError(err));
  }
}

async function handleFavoriteCommand(payload: { command: string; description: string }) {
  const pageTitle = parsedPage.value?.title || commandInput.value;
  await addToFavorites(pageTitle, payload.command, payload.description, {
    scope: lastRenderScope.value,
  });
}

// 批量复制所有命令
async function handleCopyAll() {
  if (!parsedPage.value?.examples.length) return;
  
  const allCommands = parsedPage.value.examples
    .map(ex => ex.command)
    .join('\n');
  
  try {
    await navigator.clipboard.writeText(allCommands);
    showSuccessToast(t('search.copiedAll'));
  } catch (err) {
    showErrorToast(normalizeError(err));
  }
}

// 批量收藏所有命令（仅添加未收藏的）
async function handleFavoriteAll() {
  if (!parsedPage.value?.examples.length) return;
  
  const pageTitle = parsedPage.value?.title || commandInput.value;
  const scope = lastRenderScope.value;
  const unfavorited = parsedPage.value.examples
    .filter(ex => !isFavorite(pageTitle, ex.command, scope));
  
  if (unfavorited.length === 0) {
    showErrorToast(t('settings.allAlreadyFavorited'));
    return;
  }
  
  let added = 0;
  for (const ex of unfavorited) {
    const didAdd = await addToFavorites(pageTitle, ex.command, ex.description, {
      silent: true,
      scope,
    });
    if (didAdd) {
      added++;
    }
  }
  
  if (added > 0) {
    showSuccessToast(t('settings.favoritedCount', { count: added }));
  }
}

// FavoritesPanel 事件处理
async function handleCopyFromFavorites(command: string) {
  try {
    await navigator.clipboard.writeText(command);
    showSuccessToast(t('search.copied'));
  } catch (err) {
    showErrorToast(normalizeError(err));
  }
}

async function handleRemoveFromFavorites(pageTitle: string, command: string) {
  await removeFromFavorites(pageTitle, command);
}

async function handleClearAllFavorites() {
  if (!confirm(t('settings.confirmClearAll'))) {
    return;
  }
  await clearAllFavorites();
}

// 搜索历史管理
function loadSearchHistory() {
  try {
    const saved = localStorage.getItem('tealdeer_search_history');
    if (saved) {
      searchHistory.value = JSON.parse(saved);
    }
  } catch (err) {
    logUiWarn('Failed to load search history');
  }
}

function saveSearchHistory() {
  try {
    localStorage.setItem('tealdeer_search_history', JSON.stringify(searchHistory.value));
  } catch (err) {
    logUiWarn('Failed to save search history');
  }
}

function addToSearchHistory(command: string) {
  const trimmed = command.trim();
  if (!trimmed) return;
  
  // 移除重复项
  searchHistory.value = searchHistory.value.filter(c => c !== trimmed);
  // 添加到开头
  searchHistory.value.unshift(trimmed);
  // 保持最多 10 条
  if (searchHistory.value.length > 10) {
    searchHistory.value = searchHistory.value.slice(0, 10);
  }
  saveSearchHistory();
}

// 自动补全
function updateSuggestions() {
  const input = commandInput.value.trim().toLowerCase();
  if (!input) {
    suggestions.value = searchHistory.value.slice(0, 5);
    showSuggestions.value = suggestions.value.length > 0;
    return;
  }
  
  // 模糊搜索：历史记录 + 可用命令
  const allCommands = [...new Set([...searchHistory.value, ...availableCommands.value])];
  suggestions.value = allCommands
    .filter(cmd => cmd.toLowerCase().includes(input))
    .slice(0, 8);
  
  showSuggestions.value = suggestions.value.length > 0;
  selectedSuggestionIndex.value = -1;
}

function selectSuggestion(command: string) {
  commandInput.value = command;
  showSuggestions.value = false;
  selectedSuggestionIndex.value = -1;
}

function handleCommandInputKeydown(event: KeyboardEvent) {
  if (!showSuggestions.value || suggestions.value.length === 0) return;
  
  if (event.key === 'ArrowDown') {
    event.preventDefault();
    selectedSuggestionIndex.value = Math.min(
      selectedSuggestionIndex.value + 1,
      suggestions.value.length - 1
    );
  } else if (event.key === 'ArrowUp') {
    event.preventDefault();
    selectedSuggestionIndex.value = Math.max(selectedSuggestionIndex.value - 1, -1);
  } else if (event.key === 'Enter' && selectedSuggestionIndex.value >= 0) {
    event.preventDefault();
    selectSuggestion(suggestions.value[selectedSuggestionIndex.value]);
  } else if (event.key === 'Escape') {
    showSuggestions.value = false;
    selectedSuggestionIndex.value = -1;
  }
}

// 加载可用命令列表
async function loadAvailableCommands() {
  try {
    const result = await invoke<string[]>("list_commands");
    availableCommands.value = result;
  } catch (err) {
    logUiWarn('Failed to load available commands');
  }
}

function handleCommandInputBlur() {
  setTimeout(() => {
    showSuggestions.value = false;
  }, 200);
}

async function copyRaw() {
  if (!rawOutput.value) {
    return;
  }
  try {
    await navigator.clipboard.writeText(rawOutput.value);
    showSuccessToast(t('search.copied'));
  } catch (err) {
    showErrorToast(normalizeError(err));
  }
}

function previewRaw() {
  if (rawOutput.value) {
    viewMode.value = "raw";
  }
}

function addExample() {
  examples.value.push({ desc: "", cmd: "" });
}

function removeExample(index: number) {
  if (examples.value.length > 1) {
    examples.value.splice(index, 1);
  }
}

watch(
  () => activeTab.value,
  (value) => {
    if (value === "manage") {
      loadManageEntries();
    }
    if (value === "settings") {
      loadSettings();
    }
  },
);

watch(
  () => pageScope.value,
  (value) => {
    if (value !== "all") {
      searchResults.value = [];
      searchResultsQuery.value = "";
    }
  },
);

watch(
  () => locale.value,
  (newLocale) => {
    saveLocale(newLocale);
    logUiInfo(`Language changed to: ${newLocale}`);
  },
);

watch(
  () => commandInput.value,
  () => {
    updateSuggestions();
  },
);

// Watch newPageType to reset newMode when switching to shortcut
watch(
  () => newPageType.value,
  (newType) => {
    if (newType === 'shortcut' && newMode.value === 'patch') {
      newMode.value = 'page';
    }
  },
);

onMounted(() => {
  loadAppSettings();
  resolveLogPaths();
  logUiInfo("UI mounted");
  
  // 加载搜索历史和可用命令
  loadSearchHistory();
  loadFavorites();
  loadAvailableCommands();
  
  // 监听缓存更新进度
  listen('cache-update-progress', (event) => {
    updateProgress.value = event.payload as string;
    logUiInfo(`Cache update: ${event.payload}`);
  });
  
  windowErrorHandler = (event) => {
    const location = event.filename
      ? `${event.filename}:${event.lineno ?? 0}:${event.colno ?? 0}`
      : "unknown";
    logUiError(`Window error: ${event.message} (${location})`);
  };
  windowRejectionHandler = (event) => {
    logUiError(`Unhandled rejection: ${normalizeError(event.reason)}`);
  };
  window.addEventListener("error", windowErrorHandler);
  window.addEventListener("unhandledrejection", windowRejectionHandler);
  listen<string>("tray-navigate", (event) => {
    const tab = event.payload;
    if (
      tab === "search" ||
      tab === "new" ||
      tab === "manage" ||
      tab === "settings"
    ) {
      activeTab.value = tab;
    }
  })
    .then((unlisten) => {
      unlistenNavigate = unlisten;
    })
    .catch((err) => {
      console.error("Failed to listen for tray events", err);
    });
});

  // 监听网络状态
  handleOnline = () => {
    logUiInfo("Network online");
    isOffline.value = false;
    stopOfflineBannerInterval();
  };
  
  handleOffline = () => {
    isOffline.value = true;
    startOfflineBannerInterval();
    logUiWarn("Network offline");
  };
  
  window.addEventListener("online", handleOnline);
  window.addEventListener("offline", handleOffline);
  
  // 检查初始状态
  if (!navigator.onLine) {
    handleOffline();
  }

onBeforeUnmount(() => {
  // 清理定时器
  if (commandPlaceholderTimer.value) {
    clearTimeout(commandPlaceholderTimer.value);
  }
  if (newCommandPlaceholderTimer.value) {
    clearTimeout(newCommandPlaceholderTimer.value);
  }
  
  if (unlistenNavigate) {
    unlistenNavigate();
    unlistenNavigate = null;
  }
  if (windowErrorHandler) {
    window.removeEventListener("error", windowErrorHandler);
    windowErrorHandler = null;
  }
  if (windowRejectionHandler) {
    window.removeEventListener("unhandledrejection", windowRejectionHandler);
    windowRejectionHandler = null;
  
  // 清理网络监听
  stopOfflineBannerInterval();
  if (handleOnline) {
    window.removeEventListener("online", handleOnline);
    handleOnline = null;
  }
  if (handleOffline) {
    window.removeEventListener("offline", handleOffline);
    handleOffline = null;
  }
  }
  if (themeTransitionTimer) {
    clearTimeout(themeTransitionTimer);
    themeTransitionTimer = null;
  }
});
</script>

<template>
  <main class="app">
    <!-- 离线状态横幅 -->
    <div v-if="showOfflineBanner" class="offline-banner">
      <span class="banner-icon">⚠️</span>
      <span>{{ t("common.offlineMode") }}</span>
    </div>
    <header class="hero">
      <div class="brand">
        <img class="brand-logo" :src="logoSrc" alt="TLDR logo" />
        <div>
          <h1>{{ t('app.title') }}</h1>
          <p class="subtitle">
            {{ t('app.subtitle') }}
          </p>
        </div>
      </div>
      <div class="hero-actions">
        <button
          class="immersive-toggle"
          type="button"
          title="打开沉浸式阅读"
          aria-label="打开沉浸式阅读"
          @click="openImmersiveWindow"
        >
          <span class="immersive-icon">📖</span>
        </button>
        <button
          class="theme-toggle"
          type="button"
          :title="themeToggleTitle"
          :aria-label="themeToggleTitle"
          @click="toggleTheme"
        >
          <img :src="themeIcon" alt="" class="theme-icon" aria-hidden="true" />
        </button>
      </div>
    </header>

    <nav class="tabs">
      <button
        type="button"
        :class="['tab', { active: activeTab === 'search' }]"
        @click="activeTab = 'search'"
      >
        {{ t('tabs.search') }}
      </button>
      <button
        type="button"
        :class="['tab', { active: activeTab === 'new' }]"
        @click="activeTab = 'new'"
      >
        {{ t('tabs.newPage') }}
      </button>
      <button
        type="button"
        :class="['tab', { active: activeTab === 'manage' }]"
        @click="activeTab = 'manage'"
      >
        {{ t('tabs.manage') }}
      </button>
      <button
        type="button"
        :class="['tab', { active: activeTab === 'settings' }]"
        @click="activeTab = 'settings'"
      >
        {{ t('tabs.settings') }}
      </button>
    </nav>

    <section v-if="activeTab === 'search'" class="panel form-panel">
      <div class="page-header">
        <h2>{{ t('search.title') }}</h2>
        <p>{{ t('search.description') }}</p>
      </div>
      
      <div class="page-type-selector">
        <label class="type-radio">
          <input v-model="pageScope" type="radio" value="all" />
          <span>{{ t('search.all') }}</span>
        </label>
        <label class="type-radio">
          <input v-model="pageScope" type="radio" value="command" />
          <span>{{ t('search.commands') }}</span>
        </label>
        <label class="type-radio">
          <input v-model="pageScope" type="radio" value="shortcut" />
          <span>{{ t('search.shortcuts') }}</span>
        </label>
      </div>
      
      <form class="search-form" @submit.prevent="runSearch">
        <label class="field autocomplete-wrapper">
          <span>{{ t('search.command') }}</span>
          <input
            ref="commandInputRef"
            v-model="commandInput"
            type="text"
            :placeholder="commandPlaceholder"
            :class="{ error: commandPlaceholderTemp }"
            autocomplete="off"
            @keydown="handleCommandInputKeydown"
            @focus="updateSuggestions"
            @blur="handleCommandInputBlur"
          />
          <div v-if="showSuggestions" class="suggestions">
            <div
              v-for="(suggestion, index) in suggestions"
              :key="suggestion"
              :class="['suggestion-item', { selected: index === selectedSuggestionIndex }]"
              @click="selectSuggestion(suggestion)"
            >
              {{ suggestion }}
            </div>
          </div>
        </label>

        <div class="row">
          <label class="field">
            <span>{{ t('search.language') }}</span>
            <select v-model="language">
              <option value="">{{ t('common.auto') }}</option>
              <option value="en">{{ t('common.english') }}</option>
              <option value="zh">{{ t('common.chinese') }}</option>
              <option value="ja">{{ t('common.japanese') }}</option>
              <option value="de">{{ t('common.german') }}</option>
              <option value="fr">{{ t('common.french') }}</option>
            </select>
          </label>

          <div class="field">
            <span>{{ t('search.platforms') }}</span>
            <div class="chips">
              <label
                v-for="platform in platformOptions"
                :key="platform.value"
                class="chip"
              >
                <input
                  v-model="platforms"
                  type="checkbox"
                  :value="platform.value"
                />
                <span>{{ platform.label }}</span>
              </label>
            </div>
          </div>
        </div>

        <div class="actions">
          <button class="primary" type="submit" :disabled="isLoading">
            {{ t('search.run') }}
          </button>
          <button class="ghost" type="button" @click="previewRaw" :disabled="!rawOutput">
            {{ t('search.previewRaw') }}
          </button>
          <button class="ghost" type="button" @click="copyRaw" :disabled="!rawOutput">
            {{ t('search.copy') }}
          </button>
          <button 
            class="ghost" :class="{ 'offline-state': isOffline }" 
            type="button" 
            :disabled="isLoading"
            @click="updateCache"
            :title="t('search.updateCache')"
          >
            {{ isLoading ? t('search.updating') : t('search.updateCache') }}
          </button>
          <p v-if="updateProgress" class="update-progress">{{ updateProgress }}</p>
        </div>
      </form>

      <div
        v-if="pageScope === 'all' && searchResultsQuery && searchResults.length > 0"
        class="search-results"
      >
        <div>
          <div
            v-if="groupedSearchResults.command.length"
            class="result-group"
          >
            <h3>{{ t('search.commands') }}</h3>
            <button
              v-for="entry in groupedSearchResults.command"
              :key="`command-${entry.name}-${entry.source}`"
              type="button"
              class="result-item"
              @click="selectSearchResult(entry)"
            >
              <div class="result-main">
                <span class="result-title">{{ entry.name }}</span>
                <span v-if="entry.summary" class="result-summary">{{ entry.summary }}</span>
              </div>
              <span class="result-source">{{ formatSearchSource(entry.source) }}</span>
            </button>
          </div>

          <div
            v-if="groupedSearchResults.shortcut.length"
            class="result-group"
          >
            <h3>{{ t('search.shortcuts') }}</h3>
            <button
              v-for="entry in groupedSearchResults.shortcut"
              :key="`shortcut-${entry.name}-${entry.source}`"
              type="button"
              class="result-item"
              @click="selectSearchResult(entry)"
            >
              <div class="result-main">
                <span class="result-title">{{ entry.name }}</span>
                <span v-if="entry.summary" class="result-summary">{{ entry.summary }}</span>
              </div>
              <span class="result-source">{{ formatSearchSource(entry.source) }}</span>
            </button>
          </div>
        </div>
      </div>
    </section>

    <section v-else-if="activeTab === 'new'" class="panel form-panel">
      <div class="page-header">
        <h2>{{ t('newPage.title') }}</h2>
        <p>{{ t('newPage.description') }}</p>
      </div>

      <div class="page-type-selector">
        <label>{{ t('newPage.pageType') }}:</label>
        <label class="type-radio">
          <input v-model="newPageType" type="radio" value="command" />
          <span>{{ t('newPage.commandPage') }}</span>
        </label>
        <label class="type-radio">
          <input v-model="newPageType" type="radio" value="shortcut" />
          <span>{{ t('newPage.shortcutPage') }}</span>
        </label>
      </div>

      <div class="mode-switch">
        <label class="mode">
          <input v-model="newMode" type="radio" value="page" />
          <span>{{ t('newPage.customPage') }}</span>
        </label>
        <label v-if="newPageType === 'command'" class="mode">
          <input v-model="newMode" type="radio" value="patch" />
          <span>{{ t('newPage.patch') }}</span>
          <span
            class="hint-icon"
            role="button"
            tabindex="0"
            :aria-label="t('newPage.patchTips')"
            @click.prevent
            @keydown.enter.prevent
            @keydown.space.prevent
          >
            ?
          </span>
          <span class="hint-tooltip">
            {{ t('newPage.patchTooltip') }}
          </span>
        </label>
        <label class="mode">
          <input v-model="newMode" type="radio" value="append" />
          <span>{{ t('newPage.appendExample') }}</span>
          <span
            class="hint-icon"
            role="button"
            tabindex="0"
            :aria-label="t('newPage.appendTips')"
            @click.prevent
            @keydown.enter.prevent
            @keydown.space.prevent
          >
            ?
          </span>
          <span class="hint-tooltip">
            {{ t('newPage.appendTooltip') }}
          </span>
        </label>
      </div>

      <div class="search-form">
        <label class="field">
          <span>{{ newPageType === 'shortcut' ? t('newPage.application') : t('newPage.command') }}</span>
          <input
            v-model="newCommand"
            type="text"
            :placeholder="newPageType === 'shortcut' ? t('newPage.applicationPlaceholder') : newCommandPlaceholder"
            :class="{ error: newCommandPlaceholderTemp }"
            autocomplete="off"
            
          />
        </label>

        <label v-if="newMode === 'page'" class="field">
          <span>{{ t('newPage.summary') }}</span>
          <input
            v-model="summary"
            type="text"
            :placeholder="newPageType === 'shortcut' ? t('newPage.summaryPlaceholderShortcut') : t('newPage.summaryPlaceholder')"
            autocomplete="off"
            
          />
        </label>

        <label v-if="newMode === 'patch' && newPageType === 'command'" class="toggle-field">
          <input v-model="includePatchHeader" type="checkbox" />
          <span>{{ t('newPage.includePatchHeader') }}</span>
        </label>

        <div class="examples">
          <div class="examples-header">
            <span>{{ t('newPage.examples') }}</span>
            <button
              v-if="newMode !== 'append'"
              class="ghost"
              type="button"
              @click="addExample"
            >
              {{ t('newPage.addExample') }}
            </button>
          </div>

          <div
            v-for="(example, index) in examples"
            :key="index"
            class="example-row"
            v-show="newMode !== 'append' || index === 0"
          >
            <input
              v-model="example.desc"
              type="text"
              :placeholder="newPageType === 'shortcut' ? t('newPage.descriptionPlaceholderShortcut') : t('newPage.descriptionPlaceholder')"
              
            />
            <input
              v-model="example.cmd"
              type="text"
              :placeholder="newPageType === 'shortcut' ? t('newPage.shortcutKeyPlaceholder') : t('newPage.commandPlaceholder')"
              
            />
            <button
              v-if="examples.length > 1 && newMode !== 'append'"
              class="icon"
              type="button"
              @click="removeExample(index)"
              aria-label="Remove example"
            >
              X
            </button>
          </div>
        </div>

        <div class="actions">
          <button
            class="primary"
            type="button"
            :disabled="isCreating"
            @click="createCustom"
          >
            {{ isCreating ? t('newPage.generating') : t('newPage.generate') }}
          </button>
          <button
            class="ghost"
            type="button"
            :disabled="isLoading"
            @click="runPreview"
          >
            {{ t('newPage.previewOutput') }}
          </button>
          <button class="ghost" type="button" @click="openCustomDir">
            {{ t('newPage.openCustomDir') }}
          </button>

        </div>
      </div>
    </section>

    <section v-else-if="activeTab === 'manage'" class="panel manage-panel">
      <div class="page-header">
        <h2>{{ t('manage.title') }}</h2>
        <p>{{ t('manage.description') }}</p>
      </div>

      <div class="manage-controls">
        <label class="field">
          <span>{{ t('manage.pageType') }}</span>
          <select v-model="manageFilterType">
            <option value="all">{{ t('manage.filterAll') }}</option>
            <option value="command">{{ t('manage.filterCommand') }}</option>
            <option value="shortcut">{{ t('manage.filterShortcut') }}</option>
          </select>
        </label>
        <label class="field">
          <span>{{ t('manage.type') }}</span>
          <select v-model="manageType">
            <option value="all">{{ t('common.all') }}</option>
            <option value="page">{{ t('common.page') }}</option>
            <option value="patch">{{ t('common.patch') }}</option>
          </select>
        </label>
        <label class="field">
          <span>{{ t('manage.status') }}</span>
          <select v-model="manageStatus">
            <option value="all">{{ t('common.all') }}</option>
            <option value="enabled">{{ t('common.enabled') }}</option>
            <option value="disabled">{{ t('common.disabled') }}</option>
          </select>
        </label>
        <label class="field search-field">
          <span>{{ t('manage.search') }}</span>
          <input
            v-model="manageQuery"
            type="text"
            :placeholder="t('manage.searchPlaceholder')"
          />
        </label>
        <div class="manage-actions">
          <button class="ghost" type="button" @click="loadManageEntries">
            {{ t('manage.refresh') }}
          </button>
        </div>
      </div>

      <div class="manage-body">
        <div v-if="manageLoading" class="loading">
          {{ t('manage.scanning') }}
        </div>
        <div v-else-if="filteredEntries.length === 0" class="empty">
          {{ t('manage.noPages') }}
        </div>
        <div v-else class="manage-list">
          <div v-for="entry in filteredEntries" :key="entry.path" class="manage-row">
            <div class="manage-main">
              <div class="manage-title">
                <span>{{ entry.display_command || entry.command_slug }}</span>
                <span v-if="entry.pageType" class="tag page-type">
                  {{ entry.pageType === 'shortcut' ? t('manage.typeShortcut') : t('manage.typeCommand') }}
                </span>
                <span class="tag">{{ entry.kind }}</span>
                <span class="tag" :class="entry.status">{{ entry.status }}</span>
              </div>
              <p v-if="entry.summary" class="manage-summary">{{ entry.summary }}</p>
              <div class="manage-meta">
                <span>{{ t('manage.slug') }}: {{ entry.command_slug }}</span>
                <span>{{ t('manage.examples') }}: {{ entry.examples_count }}</span>
                <span>{{ t('manage.updated') }}: {{ formatTimestamp(entry.mtime) }}</span>
              </div>
            </div>
            <div class="manage-row-actions">
              <button class="ghost" type="button" @click="openEntry(entry)">
                {{ t('manage.open') }}
              </button>
              <button class="ghost" type="button" @click="toggleEntry(entry)">
                {{ entry.status === "enabled" ? t('manage.disable') : t('manage.enable') }}
              </button>
              <button class="ghost danger" type="button" @click="deleteEntry(entry)">
                {{ t('manage.delete') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section v-else class="panel settings-panel">
      <div class="settings-header">
        <div>
          <h2>{{ t('settings.title') }}</h2>
          <p>{{ t('settings.description') }}</p>
        </div>
      </div>

      <div v-if="settingsLoading" class="loading">{{ t('settings.loadingSettings') }}</div>

      <div v-else class="settings-content">
        <!-- Content Preferences Section -->
        <div class="settings-section collapsible">
          <details open>
            <summary>
              <h3>{{ t('settings.contentPreferences') }}</h3>
            </summary>
            <div class="settings-fields">
            <label class="field">
              <span>{{ t('settings.languages') }}</span>
              <input
                v-model="settingsLanguages"
                type="text"
                :placeholder="t('settings.languagesPlaceholder')"
                
              />
            </label>

            <div class="field">
              <span>{{ t('settings.platforms') }}</span>
              <div class="chips">
                <label
                  v-for="platform in platformOptions"
                  :key="platform.value"
                  class="chip"
                >
                  <input
                    v-model="settingsPlatforms"
                    type="checkbox"
                    :value="platform.value"
                  />
                  <span>{{ platform.label }}</span>
                </label>
              </div>
            </div>

            <label class="toggle-field">
              <input v-model="settingsDisableAutoUpdate" type="checkbox" />
              <span>{{ t('settings.disableAutoUpdate') }}</span>
            </label>

            <label class="field">
              <span>{{ t('settings.updateInterval') }}</span>
              <input
                v-model.number="settingsInterval"
                type="number"
                min="1"
                
              />
            </label>

            <label class="field">
              <span>{{ t('settings.archiveSource') }}</span>
              <input
                v-model="settingsArchiveSource"
                type="text"
                :placeholder="t('settings.archiveSourcePlaceholder')"
                
              />
            </label>
            </div>
          </details>
        </div>

        <!-- Display & Interaction Section -->
        <div class="settings-section collapsible">
          <details>
            <summary>
              <h3>{{ t('settings.displayInteraction') }}</h3>
            </summary>
            <div class="settings-fields">
            <label class="field">
              <span>{{ t('settings.interfaceLanguage') }}</span>
              <select v-model="locale">
                <option value="en">English</option>
                <option value="zh">中文</option>
              </select>
            </label>

            <label class="field">
              <span>{{ t('settings.outputColor') }}</span>
              <select v-model="settingsColor">
                <option value="auto">{{ t('common.auto') }}</option>
                <option value="always">{{ t('common.always') }}</option>
                <option value="never">{{ t('common.never') }}</option>
              </select>
              <span class="field-hint">{{ t('settings.outputColorHint') }}</span>
            </label>

            <label class="toggle-field">
              <input v-model="settingsUsePager" type="checkbox" />
              <span>{{ t('settings.enablePager') }}</span>
            </label>

            <label class="field">
              <span>{{ t('settings.globalHotkey') }}</span>
              <input
                v-model="settingsHotkey"
                type="text"
                :placeholder="t('settings.globalHotkeyPlaceholder')"
                
              />
            </label>

            <label class="toggle-field">
              <input
                v-model="settingsAlwaysOnTop"
                type="checkbox"
                
              />
              <span>{{ t('settings.alwaysOnTop') }}</span>
            </label>
            </div>
          </details>
        </div>

        <!-- File Management Section -->
        <div class="settings-section collapsible">
          <details>
            <summary>
              <h3>{{ t('settings.fileManagement') }}</h3>
            </summary>
            <div class="file-actions">
              <button class="ghost" type="button" @click="openConfigFile">
                {{ t('settings.openConfig') }}
              </button>
              <button class="ghost" type="button" @click="openLogDir" :disabled="!logDir">
                {{ t('settings.openLogsFolder') }}
              </button>
              <button class="ghost" type="button" @click="openRustLog" :disabled="!logRustPath">
                {{ t('settings.openRustLog') }}
              </button>
              <button class="ghost" type="button" @click="openWebviewLog" :disabled="!logWebviewPath">
                {{ t('settings.openWebviewLog') }}
              </button>
            </div>
          </details>
        </div>

        <!-- System Information -->
        <div v-if="showPaths || logDir" class="settings-section collapsible">
          <details>
            <summary>
              <h3>{{ t('settings.systemInfo') }}</h3>
            </summary>
            <div class="system-info-grid">
              <div class="info-item"><strong>{{ t('settings.configDir') }}:</strong> {{ showPaths?.config_dir || "N/A" }}</div>
              <div class="info-item"><strong>{{ t('settings.configPath') }}:</strong> {{ showPaths?.config_path || "N/A" }}</div>
              <div class="info-item"><strong>{{ t('settings.cacheDir') }}:</strong> {{ showPaths?.cache_dir || "N/A" }}</div>
              <div class="info-item"><strong>{{ t('settings.pagesDir') }}:</strong> {{ showPaths?.pages_dir || "N/A" }}</div>
              <div class="info-item"><strong>{{ t('settings.customPagesDir') }}:</strong> {{ showPaths?.custom_pages_dir || "N/A" }}</div>
              <div class="info-item"><strong>{{ t('settings.shortcutPagesDir') }}:</strong> {{ showPaths?.shortcut_pages_dir || "N/A" }}</div>
              <div class="info-item"><strong>{{ t('settings.logDir') }}:</strong> {{ logDir || "N/A" }}</div>
              <div class="info-item"><strong>{{ t('settings.rustLog') }}:</strong> {{ logRustPath || "N/A" }}</div>
              <div class="info-item"><strong>{{ t('settings.webviewLog') }}:</strong> {{ logWebviewPath || "N/A" }}</div>
            </div>
          </details>
        </div>

        <!-- Warnings and Status -->
        <div class="settings-warnings">
          <p v-if="settingsAlwaysOnTop" class="hint">
            {{ t('settings.alwaysOnTopWarning') }}
          </p>
        </div>


        <!-- My Favorites Section -->
        <FavoritesPanel
          :favorites="favorites"
          @copy="handleCopyFromFavorites"
          @remove="handleRemoveFromFavorites"
          @clear-all="handleClearAllFavorites"
        />
        <!-- Save Button -->
        <div class="settings-save">
          <button
            class="primary save-button"
            type="button"
            @click="saveSettings"
            :disabled="settingsLoading"
          >
            {{ settingsLoading ? t('settings.savingButton') : t('settings.saveButton') }}
          </button>
        </div>
      </div>
    </section>

    <section
      v-if="activeTab === 'search' || activeTab === 'new'"
      class="panel output-panel"
    >
      <div class="output-header">
        <div>
          <h2>{{ t('search.output') }}</h2>
          <p v-if="lastCommand">{{ t('search.latest') }}: {{ lastCommand }}</p>
          <p v-else>{{ t('search.runCommand') }}</p>
        </div>
        <div class="view-toggle">
          <button
            :class="['toggle', { active: viewMode === 'rendered' }]"
            type="button"
            @click="viewMode = 'rendered'"
          >
            {{ t('search.rendered') }}
          </button>
          <button
            :class="['toggle', { active: viewMode === 'raw' }]"
            type="button"
            @click="viewMode = 'raw'"
          >
            {{ t('search.raw') }}
          </button>
        </div>
      </div>

      <div class="output-body">
        <div v-if="isLoading" class="loading">
          {{ t('search.fetching') }}
        </div>
        <div v-else-if="!rawOutput" class="empty">
          {{
            activeTab === 'search' && pageScope === 'all' && searchResultsQuery
              ? t('search.selectResult')
              : t('search.runCommand')
          }}
        </div>
        <RenderedPage
          v-if="viewMode === 'rendered' && parsedPage && !isNoResults"
          :parsed-page="parsedPage"
          :is-favorited="(cmd) => isFavorite(parsedPage?.title || commandInput, cmd, lastRenderScope)"
          :extras-html="extrasHtml"
          @copy="handleCopyCommand"
          @favorite="handleFavoriteCommand"
          @copy-all="handleCopyAll"
          @favorite-all="handleFavoriteAll"
        />
        <div
          v-else-if="viewMode === 'rendered'"
          class="markdown"
          v-html="renderedHtml"
        ></div>
        <pre v-else class="raw">{{ rawOutput }}</pre>
      </div>
    </section>

    <!-- Toast 提示 -->
    <div v-if="showToast" :class="['toast', `toast-${toastType}`]">
      <span class="toast-icon">{{ toastType === 'success' ? '✓' : '✕' }}</span>
      <span>{{ toastMessage }}</span>
    </div>
  </main>
</template>

<style>
@import "./styles/theme.css";

* {
  box-sizing: border-box;
  /* 主题切换平滑过渡 */
  transition: background-color 0.3s ease, 
              color 0.3s ease, 
              border-color 0.3s ease,
              box-shadow 0.3s ease;
}

/* 排除已有动画的元素，避免冲突 */
.theme-icon,
.primary::after,
.ghost::after,
.toast,
.panel,
.suggestion-item {
  transition: none;
}

/* 恢复这些元素的原有动画 */
.theme-icon {
  transition: transform 0.3s ease;
}

.suggestion-item {
  transition: background 0.15s ease;
}

body {
  margin: 0;
}

a {
  color: inherit;
}

mark {
  background: #fff3cd;
  color: #856404;
  padding: 2px 4px;
  border-radius: 3px;
  font-weight: 600;
}

:root[data-theme="dark"] mark {
  background: #664d03;
  color: #ffecb5;
}

button,
input,
select {
  font: inherit;
}

.app {
  min-height: 100vh;
  padding: 20px clamp(16px, 4vw, 48px) 64px;
  background:
    radial-gradient(1200px 600px at 0% 0%, var(--glow-1), transparent 60%),
    radial-gradient(1000px 500px at 100% 0%, var(--glow-2), transparent 60%),
    linear-gradient(135deg, var(--bg) 0%, var(--bg-alt) 100%);
}

.hero {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 20px;
}

.brand {
  display: flex;
  gap: 14px;
  align-items: center;
}

.hero-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.theme-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 8px;
  border-radius: 999px;
  border: 1px solid var(--button-ghost-border);
  background: var(--button-ghost-bg);
  color: var(--button-ghost-text);
  cursor: pointer;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.theme-toggle:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.theme-icon {
  width: 20px;
  height: 20px;
  display: block;
  transition: transform 0.3s ease;
}

.theme-toggle:hover .theme-icon {
  transform: rotate(15deg);
}

.immersive-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 8px;
  border-radius: 999px;
  border: 1px solid var(--button-ghost-border);
  background: var(--button-ghost-bg);
  color: var(--button-ghost-text);
  cursor: pointer;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.immersive-toggle:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.immersive-icon {
  width: 20px;
  height: 20px;
  display: block;
  transition: transform 0.3s ease;
}

.immersive-toggle:hover .immersive-icon {
  transform: scale(1.1);
}

.brand-logo {
  width: 100px;
  height: 100px;
  border-radius: 12px;
  object-fit: contain;
  flex-shrink: 0;
}

h1 {
  font-family: "Space Grotesk", sans-serif;
  font-size: clamp(1.8rem, 2.6vw, 2.4rem);
  margin: 0;
}

.hero p {
  margin: 4px 0 0;
  color: var(--text-muted-strong);
}

.subtitle {
  font-size: 0.85rem;
}

.tabs {
  display: inline-flex;
  gap: 8px;
  padding: 6px;
  background: var(--tabs-bg);
  border-radius: 999px;
  margin-bottom: 20px;
}

.tab {
  border: none;
  background: transparent;
  padding: 8px 18px;
  border-radius: 999px;
  font-weight: 600;
  cursor: pointer;
  color: var(--text-muted-strong);
}

.tab.active {
  background: var(--accent-strong);
  color: var(--text-on-strong);
}

.panel {
  background: var(--panel-bg);
  border-radius: 20px;
  padding: 24px;
  border: 1px solid var(--panel-border);
  box-shadow: var(--panel-shadow);
}

.form-panel {
  margin-bottom: 24px;
}

.search-form {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.search-results {
  margin-top: 18px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.result-group h3 {
  margin: 0 0 8px;
  font-size: 1rem;
  color: var(--text-primary);
}

.result-item {
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  padding: 12px 14px;
  border-radius: 12px;
  border: 1px solid var(--panel-border);
  background: var(--panel-bg);
  cursor: pointer;
  text-align: left;
  transition: transform 0.15s ease, box-shadow 0.15s ease;
}

.result-item:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

.result-main {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.result-title {
  font-weight: 600;
  color: var(--text-primary);
}

.result-summary {
  font-size: 0.85rem;
  color: var(--text-muted-strong);
}

.result-source {
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 8px;
  font-weight: 500;
}

.field span {
  font-size: 0.9rem;
  color: var(--text-muted-strong);
}

.field-hint {
  font-size: 0.8rem !important;
  color: var(--text-muted) !important;
  font-style: italic;
  margin-top: 4px;
}

input[type="text"],
select {
  padding: 12px 14px;
  border-radius: 12px;
  border: 1px solid var(--input-border);
  background: var(--input-bg);
  color: var(--text-primary);
}

input[type="text"]::placeholder {
  color: var(--text-muted);
}

input[type="text"].error::placeholder {
  color: var(--alert-text);
}

input[type="text"]:focus,
select:focus {
  outline: 2px solid var(--accent-outline);
  border-color: var(--accent-outline-strong);
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  transition: all 0.2s ease;
}

/* 自动补全 */
.autocomplete-wrapper {
  position: relative;
}

.suggestions {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  margin-top: 4px;
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  border-radius: 12px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  max-height: 240px;
  overflow-y: auto;
  z-index: 1000;
}

.suggestion-item {
  padding: 10px 14px;
  cursor: pointer;
  transition: background 0.15s ease;
  font-size: 0.9rem;
}

.suggestion-item:hover,
.suggestion-item.selected {
  background: var(--accent-outline);
}

.suggestion-item:first-child {
  border-radius: 12px 12px 0 0;
}

.suggestion-item:last-child {
  border-radius: 0 0 12px 12px;
}

select option,
select optgroup {
  background-color: var(--select-option-bg);
  color: var(--select-option-text);
}

.row {
  display: grid;
  grid-template-columns: minmax(200px, 260px) 1fr;
  gap: 16px;
}

.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-radius: 999px;
  border: 1px solid var(--chip-border);
  background: var(--chip-bg);
  font-size: 0.85rem;
}

.chip input {
  accent-color: var(--accent);
}

.mode-switch {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 16px;
}

.mode {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  border-radius: 999px;
  border: 1px solid var(--chip-border);
  background: var(--chip-bg);
  font-weight: 600;
  position: relative;
}

.hint-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  border: 1px solid var(--hint-icon-border);
  background: var(--hint-icon-bg);
  color: var(--hint-icon-text);
  font-size: 0.75rem;
  cursor: help;
}

.hint-tooltip {
  position: absolute;
  top: calc(100% + 8px);
  left: 50%;
  transform: translate(-50%, -4px);
  width: 240px;
  padding: 8px 10px;
  border-radius: 10px;
  background: var(--tooltip-bg);
  color: var(--tooltip-text);
  font-size: 0.8rem;
  line-height: 1.4;
  text-align: left;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.15s ease, transform 0.15s ease;
  box-shadow: var(--tooltip-shadow);
  z-index: 2;
}

.hint-icon:hover + .hint-tooltip,
.hint-icon:focus + .hint-tooltip {
  opacity: 1;
  transform: translate(-50%, 0);
}

.manage-panel {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.settings-panel {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.settings-content {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.settings-section {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.settings-section h3 {
  margin: 0;
  font-family: "Space Grotesk", sans-serif;
  font-size: 1.1rem;
  color: var(--text-primary);
  border-bottom: 1px solid var(--panel-border);
  padding-bottom: 8px;
}

.settings-section.collapsible h3 {
  border-bottom: none;
  padding-bottom: 0;
  font-size: 1rem;
}

.settings-section.collapsible details {
  border: 1px solid var(--panel-border);
  border-radius: 12px;
  padding: 16px;
  background: var(--input-bg);
}

.settings-section.collapsible summary {
  cursor: pointer;
  list-style: none;
  display: flex;
  align-items: center;
  gap: 8px;
  user-select: none;
}

.settings-section.collapsible summary::-webkit-details-marker {
  display: none;
}

.settings-section.collapsible summary::before {
  content: "▶";
  font-size: 0.8rem;
  transition: transform 0.2s ease;
  flex-shrink: 0;
}

.settings-section.collapsible details[open] summary::before {
  transform: rotate(90deg);
}

.file-actions {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
  margin-top: 16px;
}

.system-info-grid {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 16px;
}

.info-item {
  padding: 10px 12px;
  border-radius: 8px;
  background: var(--chip-bg);
  border: 1px solid var(--chip-border);
  font-size: 0.85rem;
  word-break: break-all;
}

.info-item strong {
  display: block;
  margin-bottom: 4px;
  color: var(--text-muted-strong);
  font-weight: 600;
}

.settings-warnings {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.settings-save {
  display: flex;
  justify-content: center;
  margin-top: 24px;
  padding-top: 24px;
  border-top: 1px solid var(--panel-border);
}

.save-button {
  min-width: 200px;
  max-width: 300px;
}

.page-header {
  margin-bottom: 20px;
}

.page-header h2 {
  margin: 0 0 6px 0;
  font-family: "Space Grotesk", sans-serif;
  font-size: 1.3rem;
}

.page-header p {
  margin: 0;
  color: var(--text-muted);
  font-size: 0.95rem;
}

.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.settings-header p {
  margin: 6px 0 0;
  color: var(--text-muted);
}

.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 16px;
}

.settings-fields {
  display: flex;
  flex-direction: column;
  gap: 16px;
  margin-top: 16px;
}

.paths {
  display: grid;
  gap: 6px;
  padding: 12px;
  border-radius: 12px;
  background: var(--paths-bg);
  font-size: 0.85rem;
  color: var(--text-muted-strong);
}

.paths strong {
  font-weight: 600;
}

.manage-controls {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: 12px;
  align-items: end;
}

.search-field {
  grid-column: span 2;
}

.manage-actions {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 10px;
}

.manage-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.manage-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.manage-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 16px;
  border-radius: 16px;
  border: 1px solid var(--panel-border);
  background: var(--output-bg);
}

.manage-main {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.manage-title {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  font-weight: 600;
}

.manage-summary {
  margin: 0;
  color: var(--text-muted-strong);
}

.manage-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  font-size: 0.85rem;
  color: var(--text-muted);
}

.tag {
  padding: 4px 10px;
  border-radius: 999px;
  background: var(--tag-bg);
  color: var(--text-muted-strong);
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.tag.enabled {
  background: var(--tag-enabled-bg);
  color: var(--tag-enabled-text);
}

.tag.disabled {
  background: var(--tag-disabled-bg);
  color: var(--tag-disabled-text);
}

.manage-row-actions {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.danger {
  border-color: var(--alert-border);
  color: var(--alert-text);
}

.toggle-field {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  font-weight: 600;
  color: var(--text-muted-strong);
}

.examples {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.examples-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-weight: 600;
  color: var(--text-muted-strong);
}

.example-row {
  display: grid;
  grid-template-columns: 1fr 1fr auto;
  gap: 12px;
}

.icon {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  border: 1px solid var(--icon-border);
  background: var(--icon-bg);
  cursor: pointer;
}

.actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
}

.primary,
.ghost {
  padding: 10px 18px;
  border-radius: 999px;
  border: 1px solid transparent;
  font-weight: 600;
  cursor: pointer;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.primary {
  background: var(--accent);
  color: var(--text-on-accent);
  box-shadow: var(--accent-shadow);
}

.primary:hover:not(:disabled) {
  transform: translateY(-1px);
}

.ghost {
  background: var(--button-ghost-bg);
  border-color: var(--button-ghost-border);
  color: var(--button-ghost-text);
}

.ghost:disabled,
.primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  box-shadow: none;
}

.hint {
  font-size: 0.85rem;
  color: var(--hint-text);
}

.update-progress {
  margin-top: 8px;
  font-size: 0.85rem;
  color: var(--text-muted);
  font-style: italic;
}

.status-text {
  margin: 0;
  font-weight: 600;
}

.status-text.success {
  color: var(--success-text);
}

.status-text.error {
  color: var(--alert-text);
}

.status-text.warning {
  color: var(--warning-text);
}

.error-text {
  margin: 0;
  color: var(--alert-text);
  font-weight: 600;
}

.output-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.output-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
}

h2 {
  margin: 0;
  font-family: "Space Grotesk", sans-serif;
  font-size: 1.3rem;
}

.output-header p {
  margin: 6px 0 0;
  color: var(--text-muted);
  font-size: 0.95rem;
}

.view-toggle {
  display: flex;
  background: var(--output-toggle-bg);
  border-radius: 999px;
  padding: 4px;
  gap: 6px;
}

.toggle {
  border: none;
  background: transparent;
  padding: 8px 14px;
  border-radius: 999px;
  font-weight: 600;
  cursor: pointer;
  color: var(--text-muted-strong);
}

.toggle.active {
  background: var(--accent-strong);
  color: var(--text-on-strong);
}

.output-body {
  background: var(--output-bg);
  border-radius: 16px;
  padding: 20px;
  min-height: 240px;
  max-height: 50vh;
  overflow: auto;
  border: 1px solid var(--output-border);
}

.alert {
  padding: 14px 16px;
  border-radius: 12px;
  font-weight: 500;
}

.alert.error {
  background: var(--alert-bg);
  color: var(--alert-text);
}

.alert.success {
  background: var(--success-bg);
  color: var(--success-text);
}

.alert.warning {
  background: var(--warning-bg);
  color: var(--warning-text);
}

.loading,
.empty {
  color: var(--text-muted);
  font-weight: 500;
}

.markdown :is(h1, h2, h3) {
  font-family: "Space Grotesk", sans-serif;
}

.markdown blockquote {
  margin: 0 0 12px;
  padding-left: 12px;
  border-left: 3px solid var(--accent-border);
  color: var(--text-muted-strong);
}

.markdown ul {
  padding-left: 20px;
  margin: 0 0 12px;
}

.markdown li {
  margin-bottom: 8px;
}

.markdown pre {
  margin: 0 0 16px;
}

.markdown code {
  font-family: "JetBrains Mono", monospace;
  background: var(--code-inline-bg);
  padding: 2px 6px;
  border-radius: 6px;
}

.markdown pre code {
  display: block;
  padding: 12px;
  background: var(--code-block-bg);
  color: var(--code-block-text);
}

.raw {
  font-family: "JetBrains Mono", monospace;
  white-space: pre-wrap;
  margin: 0;
}

@media (max-width: 900px) {
  .row {
    grid-template-columns: 1fr;
  }

  .search-field {
    grid-column: auto;
  }

  .manage-row {
    flex-direction: column;
  }

  .manage-row-actions {
    flex-direction: row;
    flex-wrap: wrap;
  }

  .example-row {
    grid-template-columns: 1fr;
  }

  .icon {
    width: 100%;
    border-radius: 12px;
  }
}

@media (max-width: 600px) {
  .app {
    padding: 24px 16px 48px;
  }

  .panel {
    padding: 18px;
  }
}

/* 按钮涟漪效果 */
.primary,
.ghost {
  position: relative;
  overflow: hidden;
}

.primary::after,
.ghost::after {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  width: 0;
  height: 0;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.5);
  transform: translate(-50%, -50%);
  transition: width 0.6s, height 0.6s;
}

.primary:active::after,
.ghost:active::after {
  width: 300px;
  height: 300px;
}

/* Toast 提示 */
.toast {
  position: fixed;
  top: 24px;
  right: 24px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 20px;
  border-radius: 12px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  font-weight: 600;
  z-index: 10000;
  animation: toastIn 0.3s ease-out, toastOut 0.3s ease-in 4.7s;
}

.toast-success {
  background: #d4edda;
  color: #155724;
}

.toast-error {
  background: #f8d7da;
  color: #721c24;
}

.toast-icon {
  font-size: 1.2rem;
  font-weight: bold;
}

@keyframes toastIn {
  from {
    opacity: 0;
    transform: translateY(-20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes toastOut {
  from {
    opacity: 1;
    transform: translateY(0);
  }
  to {
    opacity: 0;
    transform: translateY(-20px);
  }
}
/* 离线状态横幅 */
.offline-banner {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  background: #fff3cd;
  color: #856404;
  padding: 12px 16px;
  text-align: center;
  z-index: 1000;
  border-bottom: 2px solid #ffc107;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  animation: slideDown 0.3s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-weight: 500;
}

.banner-icon {
  font-size: 1.2rem;
}

@keyframes slideDown {
  from {
    transform: translateY(-100%);
    opacity: 0;
  }
  to {
    transform: translateY(0);
    opacity: 1;
  }
}

/* 离线状态按钮样式 */
.offline-state {
  opacity: 0.6 !important;
  cursor: not-allowed !important;
  background: #e0e0e0 !important;
  border: 2px dashed #ccc !important;
  color: #666 !important;
}

.offline-state:hover {
  transform: none !important;
  box-shadow: none !important;
}

</style>

/* Page Type Selector */
.page-type-selector {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
  padding: 8px;
  background: var(--bg-secondary);
  border-radius: 8px;
}

.page-type-selector label {
  font-weight: 600;
  margin-right: 8px;
  display: flex;
  align-items: center;
}

.type-btn {
  padding: 8px 16px;
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  color: var(--text-primary);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
}

.type-btn:hover {
  background: var(--bg-hover);
}

.type-btn.active {
  background: var(--primary-color);
  color: white;
  border-color: var(--primary-color);
}

.type-radio {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
}

.type-radio:hover {
  background: var(--bg-hover);
}

.type-radio input[type="radio"] {
  margin: 0;
}

.tag.page-type {
  background: var(--primary-color);
  color: white;
}
