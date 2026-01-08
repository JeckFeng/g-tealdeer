<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { appLogDir, join } from "@tauri-apps/api/path";
import { error as logError, info as logInfo, warn as logWarn } from "@tauri-apps/plugin-log";
import { openPath } from "@tauri-apps/plugin-opener";
import MarkdownIt from "markdown-it";
import { useI18n } from "vue-i18n";
import { saveLocale } from "./locales";
import logoLight from "../static/logo/logo_light.svg";

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
};

type ShowPaths = {
  config_dir: string | null;
  config_path: string | null;
  cache_dir: string | null;
  pages_dir: string | null;
  custom_pages_dir: string | null;
};

type ThemeMode = "light" | "dark";

type AppSettings = {
  color: string;
  hotkey_toggle: string;
  always_on_top: boolean;
  theme: ThemeMode;
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

const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: false,
});

const activeTab = ref<"search" | "new" | "manage" | "settings">("search");

const commandInput = ref("");
const language = ref("");
const platforms = ref<string[]>(["linux", "common"]);
const rawOutput = ref("");
const errorMessage = ref("");
const isLoading = ref(false);
const viewMode = ref<"rendered" | "raw">("rendered");
const copyLabel = ref(t('search.copy'));
const lastCommand = ref("");

const newMode = ref<"page" | "patch" | "append">("page");
const newCommand = ref("");
const summary = ref("");
const includePatchHeader = ref(false);
const examples = ref<ExampleInput[]>([{ desc: "", cmd: "" }]);
const newError = ref("");
const newStatus = ref("");
const newStatusType = ref<"success" | "error" | "warning">("success");
const isCreating = ref(false);

const manageEntries = ref<CustomEntry[]>([]);
const manageLoading = ref(false);
const manageError = ref("");
const manageType = ref<"all" | "page" | "patch">("all");
const manageStatus = ref<"all" | "enabled" | "disabled">("all");
const manageQuery = ref("");

const settingsLoading = ref(false);
const settingsError = ref("");
const settingsStatus = ref("");
const settingsStatusType = ref<"success" | "error" | "warning">("success");
const settingsLanguages = ref("");
const settingsPlatforms = ref<string[]>(["linux", "common"]);
const settingsDisableAutoUpdate = ref(false);
const settingsInterval = ref(24);
const settingsUsePager = ref(false);
const settingsArchiveSource = ref("");
const settingsColor = ref("auto");
const settingsHotkey = ref("Ctrl+Alt+T");
const settingsAlwaysOnTop = ref(true);
const theme = ref<ThemeMode>("light");
const showPaths = ref<ShowPaths | null>(null);
const logDir = ref<string | null>(null);
const logRustPath = ref<string | null>(null);
const logWebviewPath = ref<string | null>(null);
let unlistenNavigate: (() => void) | null = null;
let windowErrorHandler: ((event: ErrorEvent) => void) | null = null;
let windowRejectionHandler: ((event: PromiseRejectionEvent) => void) | null = null;

const platformOptions = [
  { value: "linux", label: "Linux" },
  { value: "common", label: "Common" },
  { value: "macos", label: "macOS" },
  { value: "windows", label: "Windows" },
];

const invalidReason = computed(() => validateCommandString(commandInput.value));
const previewInvalidReason = computed(() => validateCommandString(newCommand.value));
const newInvalidReason = computed(() => {
  const commandReason = validateCommandString(newCommand.value);
  if (commandReason) {
    return commandReason;
  }

  if (newMode.value === "page" && !summary.value.trim()) {
    return t("validation.summaryRequired");
  }

  const neededExamples = newMode.value === "append" ? 1 : examples.value.length;
  if (neededExamples === 0) {
    return t("validation.exampleRequired");
  }

  for (let i = 0; i < neededExamples; i += 1) {
    const example = examples.value[i];
    if (!example || !example.desc.trim() || !example.cmd.trim()) {
      return t("validation.exampleComplete");
    }
  }

  return "";
});

const filteredEntries = computed(() => {
  const query = manageQuery.value.trim().toLowerCase();
  return manageEntries.value.filter((entry) => {
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

const themeToggleLabel = computed(() =>
  theme.value === "light" ? "Light" : "Dark",
);
const themeToggleTitle = computed(() =>
  theme.value === "light" ? "Switch to dark theme" : "Switch to light theme",
);
const logoSrc = computed(() => logoLight);

const renderedHtml = computed(() => {
  if (!rawOutput.value) {
    return "";
  }
  return md.render(rawOutput.value);
});

function tokenizeCommand(value: string): string[] {
  return value.trim().split(/\s+/).filter(Boolean);
}

function validateCommandString(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) {
    return t("validation.commandRequired");
  }
  if (trimmed.includes("/") || trimmed.includes("\\") || trimmed.includes("..")) {
    return t("validation.commandInvalidPath");
  }
  return "";
}

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
  document.documentElement.dataset.theme = next;
}

function buildAppSettingsPayload(nextTheme?: ThemeMode): AppSettings {
  return {
    color: settingsColor.value || "auto",
    hotkey_toggle: settingsHotkey.value.trim(),
    always_on_top: settingsAlwaysOnTop.value,
    theme: nextTheme ?? theme.value,
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

function cleanExamples(): ExampleInput[] {
  return examples.value.map((example) => ({
    desc: example.desc.trim(),
    cmd: example.cmd.trim(),
  }));
}

function resetNewStatus() {
  newError.value = "";
  newStatus.value = "";
  newStatusType.value = "success";
}

function resetSettingsStatus() {
  settingsError.value = "";
  settingsStatus.value = "";
  settingsStatusType.value = "success";
}

function setSuccessMessage(message: string, target: "new" | "settings") {
  if (target === "new") {
    newStatus.value = message;
    newStatusType.value = "success";
    newError.value = "";
    // Auto-hide success messages after 10 seconds
    setTimeout(() => {
      if (newStatus.value === message) {
        newStatus.value = "";
      }
    }, 10000);
  } else {
    settingsStatus.value = message;
    settingsStatusType.value = "success";
    settingsError.value = "";
    // Auto-hide success messages after 10 seconds
    setTimeout(() => {
      if (settingsStatus.value === message) {
        settingsStatus.value = "";
      }
    }, 10000);
  }
}

function setErrorMessage(message: string, target: "new" | "settings") {
  if (target === "new") {
    newError.value = message;
    newStatus.value = "";
  } else {
    settingsError.value = message;
    settingsStatus.value = "";
  }
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
    const dir = await appLogDir();
    logDir.value = dir;
    logRustPath.value = await join(dir, "rust.log");
    logWebviewPath.value = await join(dir, "webview.log");
  } catch (err) {
    logUiWarn(`Failed to resolve log paths: ${normalizeError(err)}`);
  }
}

async function loadManageEntries() {
  manageLoading.value = true;
  manageError.value = "";
  try {
    const result = await invoke<CustomEntry[]>("scan_custom_pages");
    manageEntries.value = result;
  } catch (err) {
    manageError.value = normalizeError(err);
    logUiError(`Failed to scan custom pages: ${normalizeError(err)}`);
  } finally {
    manageLoading.value = false;
  }
}

async function toggleEntry(entry: CustomEntry) {
  manageError.value = "";
  manageLoading.value = true;
  try {
    if (entry.status === "enabled") {
      await invoke<CustomFileInfo>("disable_custom_file", { path: entry.path });
    } else {
      await invoke<CustomFileInfo>("enable_custom_file", { path: entry.path });
    }
    await loadManageEntries();
  } catch (err) {
    manageError.value = normalizeError(err);
    logUiError(`Failed to update custom entry: ${normalizeError(err)}`);
  } finally {
    manageLoading.value = false;
  }
}

async function deleteEntry(entry: CustomEntry) {
  manageError.value = "";
  const confirmed = window.confirm(
    `Delete ${entry.command_slug}? This cannot be undone.`,
  );
  if (!confirmed) {
    return;
  }
  manageLoading.value = true;
  try {
    await invoke("delete_custom_file", { path: entry.path });
    await loadManageEntries();
  } catch (err) {
    manageError.value = normalizeError(err);
    logUiError(`Failed to delete custom entry: ${normalizeError(err)}`);
  } finally {
    manageLoading.value = false;
  }
}

async function openEntry(entry: CustomEntry) {
  manageError.value = "";
  try {
    await openPath(entry.path);
  } catch (err) {
    manageError.value = normalizeError(err);
    logUiError(`Failed to open custom entry: ${normalizeError(err)}`);
  }
}

async function loadSettings() {
  settingsLoading.value = true;
  settingsError.value = "";
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
    settingsError.value = normalizeError(err);
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
  } catch (err) {
    settingsError.value = normalizeError(err);
    logUiError(`Failed to load app settings: ${normalizeError(err)}`);
  }
}

async function saveSettings() {
  settingsLoading.value = true;
  resetSettingsStatus();

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
    
    setSuccessMessage("Settings saved.", "settings");
    logUiInfo("Settings saved");
  } catch (err) {
    setErrorMessage(normalizeError(err), "settings");
    logUiError(`Failed to save settings: ${normalizeError(err)}`);
  } finally {
    settingsLoading.value = false;
  }
}

async function openConfigFile() {
  resetSettingsStatus();
  if (!showPaths.value?.config_path) {
    settingsError.value = "Config path is not available.";
    return;
  }
  try {
    await openPath(showPaths.value.config_path);
  } catch (err) {
    settingsError.value = normalizeError(err);
    logUiError(`Failed to open config.toml: ${normalizeError(err)}`);
  }
}

async function openLogPath(path: string | null, label: string) {
  resetSettingsStatus();
  if (!path) {
    settingsError.value = `${label} is not available.`;
    return;
  }
  try {
    await openPath(path);
  } catch (err) {
    settingsError.value = normalizeError(err);
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
  resetNewStatus();
  try {
    const paths = await invoke<ShowPaths>("get_show_paths");
    if (!paths.custom_pages_dir) {
      setErrorMessage("Custom pages directory is not available.", "new");
      return;
    }
    await openPath(paths.custom_pages_dir);
    logUiInfo("Opened custom pages directory");
  } catch (err) {
    setErrorMessage(normalizeError(err), "new");
    logUiError(`Failed to open custom pages directory: ${normalizeError(err)}`);
  }
}

async function runSearch() {
  const reason = invalidReason.value;
  if (reason) {
    errorMessage.value = reason;
    return;
  }

  isLoading.value = true;
  errorMessage.value = "";
  rawOutput.value = "";

  try {
    const command = commandInput.value.trim();
    const tokens = tokenizeCommand(command);
    logUiInfo(`Search run: ${command}`);
    const result = await invoke<RenderResult>("render_tldr", {
      commandTokens: tokens,
      language: language.value || null,
      platforms: platforms.value,
      raw: true,
      color: settingsColor.value || null,
      pager: false,
      noAutoUpdate: false,
    });
    rawOutput.value = result.stdout;
    lastCommand.value = command;
    viewMode.value = "rendered";
  } catch (err) {
    errorMessage.value = normalizeError(err);
    logUiError(`Search failed: ${normalizeError(err)}`);
  } finally {
    isLoading.value = false;
  }
}

async function updateCache() {
  isLoading.value = true;
  errorMessage.value = "";

  try {
    logUiInfo("Starting cache update");
    const result = await invoke<RenderResult>("update_cache");
    
    if (result.status === 0) {
      logUiInfo("Cache updated successfully");
      errorMessage.value = "Cache updated successfully!";
      // Set success styling for cache update message
      setTimeout(() => {
        if (errorMessage.value === "Cache updated successfully!") {
          errorMessage.value = "";
        }
      }, 10000);
    } else {
      errorMessage.value = result.stderr || "Update failed";
      logUiError(`Cache update failed: ${result.stderr}`);
    }
  } catch (err) {
    errorMessage.value = `Update failed: ${normalizeError(err)}`;
    logUiError(`Cache update error: ${normalizeError(err)}`);
  } finally {
    isLoading.value = false;
  }
}

async function runPreview() {
  const reason = previewInvalidReason.value;
  if (reason) {
    newError.value = reason;
    return;
  }

  isLoading.value = true;
  errorMessage.value = "";
  rawOutput.value = "";

  try {
    const command = newCommand.value.trim();
    const tokens = tokenizeCommand(command);
    logUiInfo(`Preview run: ${command}`);
    const result = await invoke<RenderResult>("preview_effective_output", {
      commandTokens: tokens,
      language: language.value || null,
      platforms: platforms.value,
      raw: true,
      color: settingsColor.value || null,
      pager: false,
      noAutoUpdate: false,
    });
    rawOutput.value = result.stdout;
    lastCommand.value = command;
    viewMode.value = "rendered";
  } catch (err) {
    errorMessage.value = normalizeError(err);
    logUiError(`Preview failed: ${normalizeError(err)}`);
  } finally {
    isLoading.value = false;
  }
}

async function createCustom() {
  const reason = newInvalidReason.value;
  if (reason) {
    setErrorMessage(reason, "new");
    return;
  }

  isCreating.value = true;
  resetNewStatus();

  try {
    const command = newCommand.value.trim();
    if (newMode.value === "page") {
      const req: NewPageRequest = {
        command,
        summary: summary.value.trim(),
        examples: cleanExamples(),
      };
      const result = await invoke<CustomFileInfo>("create_or_overwrite_page", {
        req,
      });
      setSuccessMessage(`Saved page to ${result.path}`, "new");
      logUiInfo(`Created custom page: ${command}`);
    } else if (newMode.value === "patch") {
      const req: NewPatchRequest = {
        command,
        examples: cleanExamples(),
        include_header_in_patch: includePatchHeader.value,
      };
      const result = await invoke<CustomFileInfo>("create_or_overwrite_patch", {
        req,
      });
      setSuccessMessage(`Saved patch to ${result.path}`, "new");
      logUiInfo(`Created custom patch: ${command}`);
    } else {
      const first = cleanExamples()[0];
      const result = await invoke<CustomFileInfo>("append_example_to_page", {
        command,
        example: first,
      });
      setSuccessMessage(`Appended example in ${result.path}`, "new");
      logUiInfo(`Appended example: ${command}`);
    }
  } catch (err) {
    setErrorMessage(normalizeError(err), "new");
    logUiError(`Failed to create custom content: ${normalizeError(err)}`);
  } finally {
    isCreating.value = false;
  }
}

async function copyRaw() {
  if (!rawOutput.value) {
    return;
  }
  try {
    await navigator.clipboard.writeText(rawOutput.value);
    copyLabel.value = t('search.copied');
    setTimeout(() => {
      copyLabel.value = t('search.copy');
    }, 1400);
  } catch (err) {
    errorMessage.value = normalizeError(err);
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
  () => locale.value,
  (newLocale) => {
    saveLocale(newLocale);
    logUiInfo(`Language changed to: ${newLocale}`);
  },
);

onMounted(() => {
  loadAppSettings();
  resolveLogPaths();
  logUiInfo("UI mounted");
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

onBeforeUnmount(() => {
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
  }
});
</script>

<template>
  <main class="app">
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
          class="theme-toggle"
          type="button"
          :title="themeToggleTitle"
          @click="toggleTheme"
        >
          <span class="theme-indicator" aria-hidden="true"></span>
          <span class="theme-label">{{ t('theme.label') }}: {{ themeToggleLabel }}</span>
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
      
      <form class="search-form" @submit.prevent="runSearch">
        <label class="field">
          <span>{{ t('search.command') }}</span>
          <input
            v-model="commandInput"
            type="text"
            :placeholder="t('search.commandPlaceholder')"
            autocomplete="off"
          />
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
          <button class="primary" type="submit" :disabled="isLoading || !!invalidReason">
            {{ isLoading ? t('search.running') : t('search.run') }}
          </button>
          <button class="ghost" type="button" @click="previewRaw" :disabled="!rawOutput">
            {{ t('search.previewRaw') }}
          </button>
          <button class="ghost" type="button" @click="copyRaw" :disabled="!rawOutput">
            {{ copyLabel }}
          </button>
          <button 
            class="ghost" 
            type="button" 
            :disabled="isLoading"
            @click="updateCache"
            :title="t('search.updateCache')"
          >
            {{ isLoading ? t('search.updating') : t('search.updateCache') }}
          </button>
          <span class="hint" v-if="invalidReason">{{ invalidReason }}</span>
        </div>
      </form>
    </section>

    <section v-else-if="activeTab === 'new'" class="panel form-panel">
      <div class="page-header">
        <h2>{{ t('newPage.title') }}</h2>
        <p>{{ t('newPage.description') }}</p>
      </div>

      <div class="mode-switch">
        <label class="mode">
          <input v-model="newMode" type="radio" value="page" />
          <span>{{ t('newPage.customPage') }}</span>
        </label>
        <label class="mode">
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
          <span>{{ t('newPage.command') }}</span>
          <input
            v-model="newCommand"
            type="text"
            :placeholder="t('search.commandPlaceholder')"
            autocomplete="off"
            @input="resetNewStatus"
          />
        </label>

        <label v-if="newMode === 'page'" class="field">
          <span>{{ t('newPage.summary') }}</span>
          <input
            v-model="summary"
            type="text"
            :placeholder="t('newPage.summaryPlaceholder')"
            autocomplete="off"
            @input="resetNewStatus"
          />
        </label>

        <label v-if="newMode === 'patch'" class="toggle-field">
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
              :placeholder="t('newPage.descriptionPlaceholder')"
              @input="resetNewStatus"
            />
            <input
              v-model="example.cmd"
              type="text"
              :placeholder="t('newPage.commandPlaceholder')"
              @input="resetNewStatus"
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
            :disabled="isCreating || !!newInvalidReason"
            @click="createCustom"
          >
            {{ isCreating ? t('newPage.generating') : t('newPage.generate') }}
          </button>
          <button
            class="ghost"
            type="button"
            :disabled="isLoading || !!previewInvalidReason"
            @click="runPreview"
          >
            {{ t('newPage.previewOutput') }}
          </button>
          <button class="ghost" type="button" @click="openCustomDir">
            {{ t('newPage.openCustomDir') }}
          </button>
          <span class="hint" v-if="newInvalidReason">{{ newInvalidReason }}</span>
        </div>

        <p v-if="newStatus" :class="['status-text', newStatusType]">{{ newStatus }}</p>
        <p v-if="newError" class="error-text">{{ newError }}</p>
      </div>
    </section>

    <section v-else-if="activeTab === 'manage'" class="panel manage-panel">
      <div class="page-header">
        <h2>{{ t('manage.title') }}</h2>
        <p>{{ t('manage.description') }}</p>
      </div>

      <div class="manage-controls">
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
        <div v-if="manageError" class="alert">
          {{ manageError }}
        </div>
        <div v-else-if="manageLoading" class="loading">
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

      <div v-if="settingsError" class="alert error">{{ settingsError }}</div>
      <div v-else-if="settingsLoading" class="loading">{{ t('settings.loadingSettings') }}</div>

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
                @input="resetSettingsStatus"
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
                @input="resetSettingsStatus"
              />
            </label>

            <label class="field">
              <span>{{ t('settings.archiveSource') }}</span>
              <input
                v-model="settingsArchiveSource"
                type="text"
                :placeholder="t('settings.archiveSourcePlaceholder')"
                @input="resetSettingsStatus"
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
                @input="resetSettingsStatus"
              />
            </label>

            <label class="toggle-field">
              <input
                v-model="settingsAlwaysOnTop"
                type="checkbox"
                @change="resetSettingsStatus"
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
          <p v-if="settingsStatus" :class="['status-text', settingsStatusType]">{{ settingsStatus }}</p>
        </div>

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
        <div v-if="errorMessage" :class="['alert', errorMessage.includes('successfully') ? 'success' : 'error']">
          {{ errorMessage }}
        </div>
        <div v-else-if="isLoading" class="loading">
          {{ t('search.fetching') }}
        </div>
        <div v-else-if="!rawOutput" class="empty">
          {{ t('search.runCommand') }}
        </div>
        <div
          v-else-if="viewMode === 'rendered'"
          class="markdown"
          v-html="renderedHtml"
        ></div>
        <pre v-else class="raw">{{ rawOutput }}</pre>
      </div>
    </section>
  </main>
</template>

<style>
@import url("https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:wght@400;500;600&family=Space+Grotesk:wght@500;600&family=JetBrains+Mono:wght@400;600&display=swap");

:root {
  font-family: "IBM Plex Sans", "Segoe UI", sans-serif;
  color: var(--text-primary);
  background-color: var(--bg);
  line-height: 1.5;
  font-weight: 400;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  color-scheme: light;
  --bg: #cbb9ff;
  --bg-alt: #f2ecff;
  --glow-1: rgba(120, 85, 255, 0.25);
  --glow-2: rgba(204, 176, 255, 0.25);
  --text-primary: #0f1f1c;
  --text-muted: #5b6662;
  --text-muted-strong: #54605b;
  --text-on-strong: #f8f6f1;
  --text-on-accent: #ffffff;
  --panel-bg: #ffffff;
  --panel-border: rgba(15, 31, 28, 0.08);
  --panel-shadow: 0 20px 50px rgba(15, 31, 28, 0.08);
  --input-bg: #f7f2ff;
  --input-border: rgba(15, 31, 28, 0.12);
  --chip-bg: #f7f2ff;
  --chip-border: rgba(15, 31, 28, 0.12);
  --accent: #2a9d8f;
  --accent-strong: #0f1f1c;
  --accent-outline: rgba(42, 157, 143, 0.4);
  --accent-outline-strong: rgba(42, 157, 143, 0.6);
  --accent-shadow: 0 12px 24px rgba(42, 157, 143, 0.25);
  --accent-border: rgba(42, 157, 143, 0.5);
  --tabs-bg: rgba(15, 31, 28, 0.08);
  --badge-bg: var(--accent-strong);
  --badge-text: var(--text-on-strong);
  --button-ghost-border: rgba(15, 31, 28, 0.16);
  --button-ghost-bg: #ffffff;
  --button-ghost-text: #2f3c38;
  --paths-bg: #f6f1ff;
  --output-bg: #fdfbff;
  --output-border: rgba(15, 31, 28, 0.08);
  --output-toggle-bg: #e9e0ff;
  --code-inline-bg: rgba(15, 31, 28, 0.08);
  --code-block-bg: #0f1f1c;
  --code-block-text: #f8f6f1;
  --alert-bg: rgba(224, 122, 95, 0.15);
  --alert-text: #8a3c28;
  --alert-border: rgba(224, 122, 95, 0.5);
  --success-bg: rgba(42, 157, 143, 0.15);
  --success-text: #1f6f64;
  --warning-bg: rgba(255, 193, 7, 0.15);
  --warning-text: #856404;
  --hint-text: #b5533a;
  --tag-bg: rgba(15, 31, 28, 0.08);
  --tag-enabled-bg: rgba(42, 157, 143, 0.15);
  --tag-enabled-text: #1f6f64;
  --tag-disabled-bg: rgba(224, 122, 95, 0.15);
  --tag-disabled-text: #8a3c28;
  --tooltip-bg: #0f1f1c;
  --tooltip-text: #f8f6f1;
  --tooltip-shadow: 0 12px 24px rgba(15, 31, 28, 0.2);
  --hint-icon-bg: #ffffff;
  --hint-icon-border: rgba(15, 31, 28, 0.25);
  --hint-icon-text: #0f1f1c;
  --icon-bg: #ffffff;
  --icon-border: rgba(15, 31, 28, 0.16);
  --select-option-bg: #ffffff;
  --select-option-text: #0f1f1c;
}

:root[data-theme="dark"] {
  color-scheme: dark;
  --bg: #050608;
  --bg-alt: #0b1224;
  --glow-1: rgba(32, 44, 88, 0.35);
  --glow-2: rgba(12, 20, 46, 0.45);
  --text-primary: #f4f6f2;
  --text-muted: #b4c0ba;
  --text-muted-strong: #c6d2cc;
  --text-on-strong: #0f1413;
  --text-on-accent: #0f1413;
  --panel-bg: #141b2b;
  --panel-border: rgba(255, 255, 255, 0.1);
  --panel-shadow: 0 18px 40px rgba(0, 0, 0, 0.4);
  --input-bg: #1b2234;
  --input-border: rgba(255, 255, 255, 0.12);
  --chip-bg: #1b2234;
  --chip-border: rgba(255, 255, 255, 0.12);
  --accent: #5cc2b6;
  --accent-strong: #f4f6f2;
  --accent-outline: rgba(92, 194, 182, 0.5);
  --accent-outline-strong: rgba(92, 194, 182, 0.75);
  --accent-shadow: 0 12px 24px rgba(92, 194, 182, 0.25);
  --accent-border: rgba(92, 194, 182, 0.5);
  --tabs-bg: rgba(255, 255, 255, 0.1);
  --badge-bg: var(--accent-strong);
  --badge-text: var(--text-on-strong);
  --button-ghost-border: rgba(255, 255, 255, 0.18);
  --button-ghost-bg: #141b2b;
  --button-ghost-text: #e4ece8;
  --paths-bg: #1b2234;
  --output-bg: #0f1524;
  --output-border: rgba(255, 255, 255, 0.08);
  --output-toggle-bg: #1b2234;
  --code-inline-bg: rgba(255, 255, 255, 0.12);
  --code-block-bg: #0b1010;
  --code-block-text: #e8f0ed;
  --alert-bg: rgba(224, 122, 95, 0.25);
  --alert-text: #f7c1b3;
  --alert-border: rgba(224, 122, 95, 0.6);
  --success-bg: rgba(92, 194, 182, 0.2);
  --success-text: #9fe5db;
  --warning-bg: rgba(255, 193, 7, 0.25);
  --warning-text: #ffeaa7;
  --hint-text: #f1a98f;
  --tag-bg: rgba(255, 255, 255, 0.12);
  --tag-enabled-bg: rgba(92, 194, 182, 0.2);
  --tag-enabled-text: #9fe5db;
  --tag-disabled-bg: rgba(224, 122, 95, 0.25);
  --tag-disabled-text: #f7c1b3;
  --tooltip-bg: #101816;
  --tooltip-text: #f4f6f2;
  --tooltip-shadow: 0 12px 24px rgba(0, 0, 0, 0.4);
  --hint-icon-bg: #1b2234;
  --hint-icon-border: rgba(255, 255, 255, 0.24);
  --hint-icon-text: #f4f6f2;
  --icon-bg: #141b2b;
  --icon-border: rgba(255, 255, 255, 0.18);
  --select-option-bg: #1b2234;
  --select-option-text: #f4f6f2;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
}

a {
  color: inherit;
}

button,
input,
select {
  font: inherit;
}

.app {
  min-height: 100vh;
  padding: 32px clamp(16px, 4vw, 48px) 64px;
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
  margin-bottom: 24px;
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
  gap: 8px;
  padding: 8px 14px;
  border-radius: 999px;
  border: 1px solid var(--button-ghost-border);
  background: var(--button-ghost-bg);
  color: var(--button-ghost-text);
  font-weight: 600;
  cursor: pointer;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.theme-toggle:hover:not(:disabled) {
  transform: translateY(-1px);
}

.theme-indicator {
  width: 10px;
  height: 10px;
  border-radius: 999px;
  background: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-outline);
}

.theme-label {
  font-size: 0.85rem;
  letter-spacing: 0.02em;
}

.brand-logo {
  width: 200px;
  height: 200px;
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

input[type="text"]:focus,
select:focus {
  outline: 2px solid var(--accent-outline);
  border-color: var(--accent-outline-strong);
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
</style>
