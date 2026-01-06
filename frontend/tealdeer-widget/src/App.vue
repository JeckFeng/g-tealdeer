<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { appLogDir, join } from "@tauri-apps/api/path";
import { error as logError, info as logInfo, warn as logWarn } from "@tauri-apps/plugin-log";
import { openPath } from "@tauri-apps/plugin-opener";
import MarkdownIt from "markdown-it";

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

type BackendBinaryInfo = {
  kind: "system" | "sidecar";
  path: string;
  version: string | null;
};

type BackendInfo = {
  system: BackendBinaryInfo | null;
  sidecar: BackendBinaryInfo | null;
  active: BackendBinaryInfo | null;
};

type AppSettings = {
  allow_system_config_write: boolean;
  color: string;
  hotkey_toggle: string;
  always_on_top: boolean;
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
const copyLabel = ref("Copy");
const lastCommand = ref("");

const newMode = ref<"page" | "patch" | "append">("page");
const newCommand = ref("");
const summary = ref("");
const includePatchHeader = ref(false);
const examples = ref<ExampleInput[]>([{ desc: "", cmd: "" }]);
const newError = ref("");
const newStatus = ref("");
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
const settingsLanguages = ref("");
const settingsPlatforms = ref<string[]>(["linux", "common"]);
const settingsAutoUpdate = ref(false);
const settingsInterval = ref(24);
const settingsUsePager = ref(false);
const settingsArchiveSource = ref("");
const settingsColor = ref("auto");
const settingsHotkey = ref("Ctrl+Alt+T");
const settingsAlwaysOnTop = ref(true);
const allowSystemConfigWrite = ref(false);
const showPaths = ref<ShowPaths | null>(null);
const backendInfo = ref<BackendInfo | null>(null);
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
    return "Summary is required for a custom page.";
  }

  const neededExamples = newMode.value === "append" ? 1 : examples.value.length;
  if (neededExamples === 0) {
    return "At least one example is required.";
  }

  for (let i = 0; i < neededExamples; i += 1) {
    const example = examples.value[i];
    if (!example || !example.desc.trim() || !example.cmd.trim()) {
      return "Each example needs a description and a command.";
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

const isSystemBackend = computed(
  () => backendInfo.value?.active?.kind === "system",
);
const canWriteConfig = computed(
  () => !isSystemBackend.value || allowSystemConfigWrite.value,
);

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
    return "Command is required.";
  }
  if (trimmed.includes("/") || trimmed.includes("\\") || trimmed.includes("..")) {
    return "Command contains a path separator or traversal segment.";
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

function cleanExamples(): ExampleInput[] {
  return examples.value.map((example) => ({
    desc: example.desc.trim(),
    cmd: example.cmd.trim(),
  }));
}

function resetNewStatus() {
  newError.value = "";
  newStatus.value = "";
}

function resetSettingsStatus() {
  settingsError.value = "";
  settingsStatus.value = "";
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
    const [appSettings, configValues, paths, backend] = await Promise.all([
      invoke<AppSettings>("get_app_settings"),
      invoke<TealdeerConfigValues>("get_tealdeer_config_values"),
      invoke<ShowPaths>("get_show_paths"),
      invoke<BackendInfo>("detect_backend"),
    ]);

    allowSystemConfigWrite.value = appSettings.allow_system_config_write;
    settingsColor.value = appSettings.color || "auto";
    settingsHotkey.value = appSettings.hotkey_toggle;
    settingsAlwaysOnTop.value = appSettings.always_on_top;

    settingsLanguages.value = configValues.languages.join(", ");
    settingsPlatforms.value =
      configValues.platforms.length > 0
        ? configValues.platforms
        : ["linux", "common"];
    settingsAutoUpdate.value = configValues.auto_update;
    settingsInterval.value = configValues.auto_update_interval_hours ?? 24;
    settingsUsePager.value = configValues.use_pager;
    settingsArchiveSource.value = configValues.archive_source ?? "";
    showPaths.value = paths;
    backendInfo.value = backend;
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
    allowSystemConfigWrite.value = appSettings.allow_system_config_write;
    settingsColor.value = appSettings.color || "auto";
    settingsHotkey.value = appSettings.hotkey_toggle;
    settingsAlwaysOnTop.value = appSettings.always_on_top;
  } catch (err) {
    settingsError.value = normalizeError(err);
    logUiError(`Failed to load app settings: ${normalizeError(err)}`);
  }
}

async function saveSettings() {
  settingsLoading.value = true;
  resetSettingsStatus();

  try {
    const appSettings: AppSettings = {
      allow_system_config_write: allowSystemConfigWrite.value,
      color: settingsColor.value || "auto",
      hotkey_toggle: settingsHotkey.value.trim(),
      always_on_top: settingsAlwaysOnTop.value,
    };
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
      auto_update: settingsAutoUpdate.value,
      auto_update_interval_hours: interval,
      use_pager: settingsUsePager.value,
      archive_source: settingsArchiveSource.value.trim()
        ? settingsArchiveSource.value.trim()
        : null,
    };
    await invoke("set_tealdeer_config", { patch });
    settingsStatus.value = "Settings saved.";
    logUiInfo("Settings saved");
  } catch (err) {
    settingsError.value = normalizeError(err);
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

async function refreshShowPaths() {
  resetSettingsStatus();
  try {
    showPaths.value = await invoke<ShowPaths>("get_show_paths");
  } catch (err) {
    settingsError.value = normalizeError(err);
    logUiError(`Failed to refresh paths: ${normalizeError(err)}`);
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
    newError.value = reason;
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
      newStatus.value = `Saved page to ${result.path}`;
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
      newStatus.value = `Saved patch to ${result.path}`;
      logUiInfo(`Created custom patch: ${command}`);
    } else {
      const first = cleanExamples()[0];
      const result = await invoke<CustomFileInfo>("append_example_to_page", {
        command,
        example: first,
      });
      newStatus.value = `Appended example in ${result.path}`;
      logUiInfo(`Appended example: ${command}`);
    }
  } catch (err) {
    newError.value = normalizeError(err);
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
    copyLabel.value = "Copied";
    setTimeout(() => {
      copyLabel.value = "Copy";
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
        <div class="badge">TLDR</div>
        <div>
          <h1>Tealdeer Widget</h1>
          <p>Search and curate tldr pages in a compact Linux window.</p>
        </div>
      </div>
      <div class="status">
        <span class="pill">Linux</span>
        <span class="pill">Tauri v2</span>
        <span class="pill ghost">Markdown</span>
      </div>
    </header>

    <nav class="tabs">
      <button
        type="button"
        :class="['tab', { active: activeTab === 'search' }]"
        @click="activeTab = 'search'"
      >
        Search
      </button>
      <button
        type="button"
        :class="['tab', { active: activeTab === 'new' }]"
        @click="activeTab = 'new'"
      >
        New Page
      </button>
      <button
        type="button"
        :class="['tab', { active: activeTab === 'manage' }]"
        @click="activeTab = 'manage'"
      >
        Manage
      </button>
      <button
        type="button"
        :class="['tab', { active: activeTab === 'settings' }]"
        @click="activeTab = 'settings'"
      >
        Settings
      </button>
    </nav>

    <section v-if="activeTab === 'search'" class="panel form-panel">
      <form class="search-form" @submit.prevent="runSearch">
        <label class="field">
          <span>Command</span>
          <input
            v-model="commandInput"
            type="text"
            placeholder="git log"
            autocomplete="off"
          />
        </label>

        <div class="row">
          <label class="field">
            <span>Language</span>
            <select v-model="language">
              <option value="">Auto</option>
              <option value="en">English</option>
              <option value="zh">Chinese</option>
              <option value="ja">Japanese</option>
              <option value="de">German</option>
              <option value="fr">French</option>
            </select>
          </label>

          <div class="field">
            <span>Platforms</span>
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
            {{ isLoading ? "Running..." : "Run" }}
          </button>
          <button class="ghost" type="button" @click="previewRaw" :disabled="!rawOutput">
            Preview Raw
          </button>
          <button class="ghost" type="button" @click="copyRaw" :disabled="!rawOutput">
            {{ copyLabel }}
          </button>
          <button class="ghost" type="button" disabled title="Planned for a later stage">
            Update Cache
          </button>
          <span class="hint" v-if="invalidReason">{{ invalidReason }}</span>
        </div>
      </form>
    </section>

    <section v-else-if="activeTab === 'new'" class="panel form-panel">
      <div class="mode-switch">
        <label class="mode">
          <input v-model="newMode" type="radio" value="page" />
          <span>Custom Page</span>
        </label>
        <label class="mode">
          <input v-model="newMode" type="radio" value="patch" />
          <span>Patch</span>
        </label>
        <label class="mode">
          <input v-model="newMode" type="radio" value="append" />
          <span>Append Example</span>
        </label>
      </div>

      <div class="search-form">
        <label class="field">
          <span>Command</span>
          <input
            v-model="newCommand"
            type="text"
            placeholder="git log"
            autocomplete="off"
            @input="resetNewStatus"
          />
        </label>

        <label v-if="newMode === 'page'" class="field">
          <span>Summary</span>
          <input
            v-model="summary"
            type="text"
            placeholder="One line summary"
            autocomplete="off"
            @input="resetNewStatus"
          />
        </label>

        <label v-if="newMode === 'patch'" class="toggle-field">
          <input v-model="includePatchHeader" type="checkbox" />
          <span>Include header in patch</span>
        </label>

        <div class="examples">
          <div class="examples-header">
            <span>Examples</span>
            <button
              v-if="newMode !== 'append'"
              class="ghost"
              type="button"
              @click="addExample"
            >
              Add Example
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
              placeholder="Description"
              @input="resetNewStatus"
            />
            <input
              v-model="example.cmd"
              type="text"
              placeholder="Command"
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
            {{ isCreating ? "Saving..." : "Generate" }}
          </button>
          <button
            class="ghost"
            type="button"
            :disabled="isLoading || !!previewInvalidReason"
            @click="runPreview"
          >
            Preview Effective Output
          </button>
          <button class="ghost" type="button" disabled title="Planned for a later stage">
            Open Custom Dir
          </button>
          <span class="hint" v-if="newInvalidReason">{{ newInvalidReason }}</span>
        </div>

        <p v-if="newStatus" class="status-text">{{ newStatus }}</p>
        <p v-if="newError" class="error-text">{{ newError }}</p>
      </div>
    </section>

    <section v-else-if="activeTab === 'manage'" class="panel manage-panel">
      <div class="manage-controls">
        <label class="field">
          <span>Type</span>
          <select v-model="manageType">
            <option value="all">All</option>
            <option value="page">Page</option>
            <option value="patch">Patch</option>
          </select>
        </label>
        <label class="field">
          <span>Status</span>
          <select v-model="manageStatus">
            <option value="all">All</option>
            <option value="enabled">Enabled</option>
            <option value="disabled">Disabled</option>
          </select>
        </label>
        <label class="field search-field">
          <span>Search</span>
          <input
            v-model="manageQuery"
            type="text"
            placeholder="Filter by command or summary"
          />
        </label>
        <div class="manage-actions">
          <button class="ghost" type="button" @click="loadManageEntries">
            Refresh
          </button>
        </div>
      </div>

      <div class="manage-body">
        <div v-if="manageError" class="alert">
          {{ manageError }}
        </div>
        <div v-else-if="manageLoading" class="loading">
          Scanning custom pages...
        </div>
        <div v-else-if="filteredEntries.length === 0" class="empty">
          No custom pages found.
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
                <span>Slug: {{ entry.command_slug }}</span>
                <span>Examples: {{ entry.examples_count }}</span>
                <span>Updated: {{ formatTimestamp(entry.mtime) }}</span>
              </div>
            </div>
            <div class="manage-row-actions">
              <button class="ghost" type="button" @click="openEntry(entry)">
                Open
              </button>
              <button class="ghost" type="button" @click="toggleEntry(entry)">
                {{ entry.status === "enabled" ? "Disable" : "Enable" }}
              </button>
              <button class="ghost danger" type="button" @click="deleteEntry(entry)">
                Delete
              </button>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section v-else class="panel settings-panel">
      <div class="settings-header">
        <div>
          <h2>Settings</h2>
          <p>Manage tealdeer configuration and app defaults.</p>
        </div>
        <div class="settings-status">
          <span v-if="backendInfo?.active" class="pill ghost">
            Active: {{ backendInfo.active.kind }}
          </span>
        </div>
      </div>

      <div v-if="settingsError" class="alert">{{ settingsError }}</div>
      <div v-else-if="settingsLoading" class="loading">Loading settings...</div>

      <div v-else class="settings-grid">
        <label class="field">
          <span>Languages (comma-separated)</span>
          <input
            v-model="settingsLanguages"
            type="text"
            placeholder="en, zh"
            @input="resetSettingsStatus"
          />
        </label>

        <div class="field">
          <span>Platforms</span>
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
          <input v-model="settingsAutoUpdate" type="checkbox" />
          <span>Auto update cache</span>
        </label>

        <label class="field">
          <span>Update interval (hours)</span>
          <input
            v-model.number="settingsInterval"
            type="number"
            min="1"
            @input="resetSettingsStatus"
          />
        </label>

        <label class="toggle-field">
          <input v-model="settingsUsePager" type="checkbox" />
          <span>Enable pager in tealdeer</span>
        </label>

        <label class="field">
          <span>Archive source</span>
          <input
            v-model="settingsArchiveSource"
            type="text"
            placeholder="https://github.com/tldr-pages/tldr/releases/latest/download/"
            @input="resetSettingsStatus"
          />
        </label>

        <label class="field">
          <span>Output color</span>
          <select v-model="settingsColor">
            <option value="auto">Auto</option>
            <option value="always">Always</option>
            <option value="never">Never</option>
          </select>
        </label>

        <label class="field">
          <span>Global hotkey (leave blank to disable)</span>
          <input
            v-model="settingsHotkey"
            type="text"
            placeholder="Ctrl+Alt+T"
            @input="resetSettingsStatus"
          />
        </label>

        <label class="toggle-field">
          <input
            v-model="settingsAlwaysOnTop"
            type="checkbox"
            @change="resetSettingsStatus"
          />
          <span>Always on top</span>
        </label>

        <label class="toggle-field">
          <input v-model="allowSystemConfigWrite" type="checkbox" />
          <span>Allow editing system tealdeer config</span>
        </label>
      </div>

      <div class="actions">
        <button
          class="primary"
          type="button"
          @click="saveSettings"
          :disabled="settingsLoading || !canWriteConfig"
        >
          Save Settings
        </button>
        <button class="ghost" type="button" @click="openConfigFile">
          Open config.toml
        </button>
        <button class="ghost" type="button" @click="refreshShowPaths">
          Show paths
        </button>
        <button class="ghost" type="button" @click="openLogDir" :disabled="!logDir">
          Open logs folder
        </button>
        <button class="ghost" type="button" @click="openRustLog" :disabled="!logRustPath">
          Open rust.log
        </button>
        <button class="ghost" type="button" @click="openWebviewLog" :disabled="!logWebviewPath">
          Open webview.log
        </button>
      </div>

      <p v-if="!canWriteConfig" class="hint">
        System config is read-only. Enable the toggle above to allow edits.
      </p>
      <p class="hint">
        Always-on-top can be ignored on some Wayland desktops; disable it if stacking feels
        unstable.
      </p>
      <p v-if="settingsStatus" class="status-text">{{ settingsStatus }}</p>

      <div v-if="showPaths || logDir" class="paths">
        <div><strong>Config dir:</strong> {{ showPaths?.config_dir || "N/A" }}</div>
        <div><strong>Config path:</strong> {{ showPaths?.config_path || "N/A" }}</div>
        <div><strong>Cache dir:</strong> {{ showPaths?.cache_dir || "N/A" }}</div>
        <div><strong>Pages dir:</strong> {{ showPaths?.pages_dir || "N/A" }}</div>
        <div><strong>Custom pages dir:</strong> {{ showPaths?.custom_pages_dir || "N/A" }}</div>
        <div><strong>Log dir:</strong> {{ logDir || "N/A" }}</div>
        <div><strong>Rust log:</strong> {{ logRustPath || "N/A" }}</div>
        <div><strong>Webview log:</strong> {{ logWebviewPath || "N/A" }}</div>
      </div>
    </section>

    <section class="panel output-panel">
      <div class="output-header">
        <div>
          <h2>Output</h2>
          <p v-if="lastCommand">Latest: {{ lastCommand }}</p>
          <p v-else>Run a command to see the rendered page.</p>
        </div>
        <div class="view-toggle">
          <button
            :class="['toggle', { active: viewMode === 'rendered' }]"
            type="button"
            @click="viewMode = 'rendered'"
          >
            Rendered
          </button>
          <button
            :class="['toggle', { active: viewMode === 'raw' }]"
            type="button"
            @click="viewMode = 'raw'"
          >
            Raw
          </button>
        </div>
      </div>

      <div class="output-body">
        <div v-if="errorMessage" class="alert">
          {{ errorMessage }}
        </div>
        <div v-else-if="isLoading" class="loading">
          Fetching page...
        </div>
        <div v-else-if="!rawOutput" class="empty">
          The rendered content will appear here.
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
  color: #0f1f1c;
  background-color: #f2f0ea;
  line-height: 1.5;
  font-weight: 400;
  text-rendering: optimizeLegibility;
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
    radial-gradient(1200px 600px at 0% 0%, rgba(42, 157, 143, 0.15), transparent 60%),
    radial-gradient(1000px 500px at 100% 0%, rgba(224, 122, 95, 0.15), transparent 60%),
    linear-gradient(135deg, #f2f0ea 0%, #edf6f6 100%);
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
  gap: 16px;
  align-items: center;
}

.badge {
  font-family: "Space Grotesk", sans-serif;
  font-weight: 600;
  font-size: 1.1rem;
  padding: 10px 14px;
  border-radius: 12px;
  background: #0f1f1c;
  color: #f8f6f1;
  letter-spacing: 0.08em;
}

h1 {
  font-family: "Space Grotesk", sans-serif;
  font-size: clamp(1.8rem, 2.6vw, 2.4rem);
  margin: 0;
}

.hero p {
  margin: 4px 0 0;
  color: #54605b;
}

.status {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.pill {
  padding: 6px 12px;
  border-radius: 999px;
  background: rgba(15, 31, 28, 0.1);
  color: #0f1f1c;
  font-size: 0.85rem;
  font-weight: 500;
}

.pill.ghost {
  background: transparent;
  border: 1px solid rgba(15, 31, 28, 0.2);
}

.tabs {
  display: inline-flex;
  gap: 8px;
  padding: 6px;
  background: rgba(15, 31, 28, 0.08);
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
  color: #3a4642;
}

.tab.active {
  background: #0f1f1c;
  color: #f8f6f1;
}

.panel {
  background: #ffffff;
  border-radius: 20px;
  padding: 24px;
  border: 1px solid rgba(15, 31, 28, 0.08);
  box-shadow: 0 20px 50px rgba(15, 31, 28, 0.08);
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
  color: #3a4642;
}

input[type="text"],
select {
  padding: 12px 14px;
  border-radius: 12px;
  border: 1px solid rgba(15, 31, 28, 0.16);
  background: #fdfbf7;
}

input[type="text"]:focus,
select:focus {
  outline: 2px solid rgba(42, 157, 143, 0.4);
  border-color: rgba(42, 157, 143, 0.6);
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
  border: 1px solid rgba(15, 31, 28, 0.12);
  background: #f6f3ee;
  font-size: 0.85rem;
}

.chip input {
  accent-color: #2a9d8f;
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
  border: 1px solid rgba(15, 31, 28, 0.12);
  background: #f6f3ee;
  font-weight: 600;
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

.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.settings-header p {
  margin: 6px 0 0;
  color: #5b6662;
}

.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 16px;
}

.paths {
  display: grid;
  gap: 6px;
  padding: 12px;
  border-radius: 12px;
  background: #f6f3ee;
  font-size: 0.85rem;
  color: #4f5b57;
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
  border: 1px solid rgba(15, 31, 28, 0.12);
  background: #fdfbf7;
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
  color: #4f5b57;
}

.manage-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  font-size: 0.85rem;
  color: #5b6662;
}

.tag {
  padding: 4px 10px;
  border-radius: 999px;
  background: rgba(15, 31, 28, 0.08);
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.tag.enabled {
  background: rgba(42, 157, 143, 0.15);
  color: #1f6f64;
}

.tag.disabled {
  background: rgba(224, 122, 95, 0.15);
  color: #8a3c28;
}

.manage-row-actions {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.danger {
  border-color: rgba(224, 122, 95, 0.5);
  color: #8a3c28;
}

.toggle-field {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  font-weight: 600;
  color: #3a4642;
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
  color: #3a4642;
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
  border: 1px solid rgba(15, 31, 28, 0.16);
  background: #ffffff;
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
  background: #2a9d8f;
  color: #ffffff;
  box-shadow: 0 12px 24px rgba(42, 157, 143, 0.25);
}

.primary:hover:not(:disabled) {
  transform: translateY(-1px);
}

.ghost {
  background: #ffffff;
  border-color: rgba(15, 31, 28, 0.16);
  color: #2f3c38;
}

.ghost:disabled,
.primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  box-shadow: none;
}

.hint {
  font-size: 0.85rem;
  color: #b5533a;
}

.status-text {
  margin: 0;
  color: #2a9d8f;
  font-weight: 600;
}

.error-text {
  margin: 0;
  color: #b5533a;
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
  color: #5b6662;
  font-size: 0.95rem;
}

.view-toggle {
  display: flex;
  background: #f2f0ea;
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
  color: #4a5551;
}

.toggle.active {
  background: #0f1f1c;
  color: #f8f6f1;
}

.output-body {
  background: #fdfbf7;
  border-radius: 16px;
  padding: 20px;
  min-height: 240px;
  border: 1px solid rgba(15, 31, 28, 0.08);
}

.alert {
  padding: 14px 16px;
  border-radius: 12px;
  background: rgba(224, 122, 95, 0.15);
  color: #8a3c28;
  font-weight: 500;
}

.loading,
.empty {
  color: #5e6a66;
  font-weight: 500;
}

.markdown :is(h1, h2, h3) {
  font-family: "Space Grotesk", sans-serif;
}

.markdown blockquote {
  margin: 0 0 12px;
  padding-left: 12px;
  border-left: 3px solid rgba(42, 157, 143, 0.5);
  color: #4f5b57;
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
  background: rgba(15, 31, 28, 0.08);
  padding: 2px 6px;
  border-radius: 6px;
}

.markdown pre code {
  display: block;
  padding: 12px;
  background: #0f1f1c;
  color: #f8f6f1;
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
