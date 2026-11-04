<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { api } from './api';
import type {
  Group,
  RelayNode,
  Preferences,
  ConfigDirInfo,
  ClaudeBinaryInfo,
  NodeTestResult,
  ConfigFileItem,
  OsInfo,
} from './types';

// Navigation View: 'main' | 'settings'
const currentView = ref<'main' | 'settings'>('main');

// State
const configDirInfo = ref<ConfigDirInfo | null>(null);
const claudeBinary = ref<ClaudeBinaryInfo | null>(null);
const preferences = ref<Preferences>({
  configDirMode: 'auto',
  customClaudeConfigDir: null,
  terminal: 'system',
  claudeExecutablePath: null,
  lastProjectDir: null,
});
const groups = ref<Group[]>([]);
const nodes = ref<RelayNode[]>([]);
const currentGroupId = ref<string>('');
const configFiles = ref<ConfigFileItem[]>([]);

// Host OS Detection
const hostOs = ref<OsInfo>({
  os: 'windows',
  isWindows: true,
  isMacos: false,
  userHome: '',
  defaultClaudeDirRepr: '%USERPROFILE%\\.claude\\',
});

// Settings view specific state (auto-detected on startup!)
const selectedOsPreview = ref<'macOS' | 'Windows'>('Windows');
const customDirInput = ref('');

// Testing State
const nodeTestingState = ref<Record<string, { testing: boolean; result?: NodeTestResult }>>({});
const isTestingAll = ref(false);

// Notification & Copy feedback
const copiedTooltip = ref(false);
const statusBanner = ref<{ type: 'success' | 'warning' | 'error' | 'info'; text: string } | null>(null);

// Modals
const showCreateGroupModal = ref(false);
const showNodeModal = ref(false);
const showJsonModal = ref(false);

// Group form
const newGroupName = ref('');
const newGroupSlug = ref('');
const newGroupInitialNodeId = ref('');

// Node form
const editingNode = ref<{
  isNew: boolean;
  node: RelayNode;
}>({
  isNew: true,
  node: {
    id: '',
    name: '',
    baseUrl: '',
    authMode: 'authToken',
    authToken: '',
    models: {
      opus: 'claude-3-opus-20240229',
      sonnet: 'claude-3-7-sonnet-20250219',
      haiku: 'claude-3-5-haiku-20241022',
      defaultModel: 'claude-3-7-sonnet-20250219',
    },
  },
});
const showPassword = ref(false);
const currentJsonContent = ref('');
const currentEditingFilename = ref('');

// Computed
const currentGroup = computed(() => {
  return groups.value.find((g) => g.id === currentGroupId.value) || null;
});

const activeNodeForCurrentGroup = computed(() => {
  if (!currentGroup.value?.activeNodeId) return null;
  return nodes.value.find((n) => n.id === currentGroup.value?.activeNodeId) || null;
});

// Whether the currently active target OS is Windows
const isCurrentWindows = computed(() => {
  return selectedOsPreview.value === 'Windows';
});

// Dynamic settings file path representation based on OS
const displaySettingsPath = computed(() => {
  const filename = currentGroup.value ? currentGroup.value.settingsFile : 'my-opus-4.6.json';

  // Custom directory mode
  if (preferences.value.configDirMode === 'custom' && customDirInput.value.trim()) {
    const dir = customDirInput.value.trim().replace(/[/\\]+$/, '');
    const sep = isCurrentWindows.value ? '\\' : '/';
    return `"${dir}${sep}${filename}"`;
  }

  // Auto/Default mode: dynamically formatted for Windows or macOS
  if (isCurrentWindows.value) {
    return `%USERPROFILE%\\.claude\\${filename}`;
  } else {
    return `~/.claude/${filename}`;
  }
});

// Command preview dynamically formatted for Windows / macOS
const previewCommand = computed(() => {
  return `claude --settings ${displaySettingsPath.value}`;
});

// Settings page directory preview path based on selected OS preview
const settingsDirDisplay = computed(() => {
  if (preferences.value.configDirMode === 'custom' && customDirInput.value.trim()) {
    return customDirInput.value.trim();
  }
  if (selectedOsPreview.value === 'macOS') {
    return '~/.claude/';
  } else {
    return configDirInfo.value?.resolvedPath || '%USERPROFILE%\\.claude\\';
  }
});

function toggleOsPreview() {
  selectedOsPreview.value = selectedOsPreview.value === 'Windows' ? 'macOS' : 'Windows';
  notify(`已切换预览系统为: ${selectedOsPreview.value}`, 'info', 1500);
}

function notify(text: string, type: 'success' | 'warning' | 'error' | 'info' = 'info', ms = 3000) {
  statusBanner.value = { text, type };
  if (ms > 0) {
    setTimeout(() => {
      if (statusBanner.value?.text === text) {
        statusBanner.value = null;
      }
    }, ms);
  }
}

onMounted(async () => {
  await loadData();
});

async function loadData() {
  try {
    const [cfg, allNodes, dirInfo, binaryInfo, files, os] = await Promise.all([
      api.getLauncherConfig(),
      api.getNodes(),
      api.getClaudeConfigDirInfo(),
      api.detectClaudeBinary(),
      api.listConfigFiles().catch(() => []),
      api.getOsInfo().catch(() => ({
        os: 'windows',
        isWindows: true,
        isMacos: false,
        userHome: '',
        defaultClaudeDirRepr: '%USERPROFILE%\\.claude\\',
      })),
    ]);

    groups.value = cfg.groups || [];
    nodes.value = allNodes || [];
    preferences.value = cfg.preferences || {
      configDirMode: 'auto',
      customClaudeConfigDir: null,
      terminal: 'system',
      claudeExecutablePath: null,
      lastProjectDir: null,
    };
    customDirInput.value = cfg.preferences?.customClaudeConfigDir || '';
    configDirInfo.value = dirInfo;
    claudeBinary.value = binaryInfo;
    configFiles.value = files || [];
    hostOs.value = os;

    // Automatically set OS based on real host platform!
    selectedOsPreview.value = os.isWindows ? 'Windows' : 'macOS';

    if (groups.value.length > 0) {
      if (!currentGroupId.value || !groups.value.some((g) => g.id === currentGroupId.value)) {
        currentGroupId.value = groups.value[0].id;
      }
    } else {
      currentGroupId.value = '';
    }
  } catch (err: any) {
    notify(`加载数据失败: ${err}`, 'error');
  }
}

// Group Actions
function openAddGroup() {
  newGroupName.value = '';
  newGroupSlug.value = '';
  newGroupInitialNodeId.value = nodes.value[0]?.id || '';
  showCreateGroupModal.value = true;
}

function onGroupNameChange() {
  const raw = newGroupName.value.toLowerCase().trim();
  newGroupSlug.value = raw
    .replace(/[^a-z0-9-_.]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
}

async function handleCreateGroup() {
  if (!newGroupName.value.trim()) {
    notify('请输入模型分组名称', 'warning');
    return;
  }
  try {
    const newGroup = await api.createGroup(
      newGroupName.value.trim(),
      newGroupSlug.value.trim() || undefined,
      newGroupInitialNodeId.value || undefined
    );
    groups.value.push(newGroup);
    currentGroupId.value = newGroup.id;
    showCreateGroupModal.value = false;
    notify(`已创建分组「${newGroup.name}」(${newGroup.settingsFile})`, 'success');
    configFiles.value = await api.listConfigFiles().catch(() => []);
  } catch (err: any) {
    notify(`创建分组失败: ${err}`, 'error', 5000);
  }
}

async function handleDeleteCurrentGroup() {
  if (!currentGroup.value) return;
  const ok = confirm(`确定删除模型分组「${currentGroup.value.name}」吗？\n配置文件 ${currentGroup.value.settingsFile} 也将被移除。`);
  if (!ok) return;

  try {
    const gid = currentGroup.value.id;
    await api.deleteGroup(gid, true);
    groups.value = groups.value.filter((g) => g.id !== gid);
    currentGroupId.value = groups.value[0]?.id || '';
    configFiles.value = await api.listConfigFiles().catch(() => []);
    notify('分组已删除', 'info');
  } catch (err: any) {
    notify(`删除失败: ${err}`, 'error');
  }
}

// Node Actions
async function handleSwitchNode(nodeId: string) {
  if (!currentGroup.value) {
    notify('请先在上方新建或选择模型分组', 'warning');
    return;
  }
  if (currentGroup.value.activeNodeId === nodeId) return;

  try {
    await api.switchGroupNode(currentGroup.value.id, nodeId);
    currentGroup.value.activeNodeId = nodeId;
    notify(`已切换至节点「${nodes.value.find((n) => n.id === nodeId)?.name}」`, 'success');
  } catch (err: any) {
    notify(`切换节点失败: ${err}`, 'error');
  }
}

function openAddNode() {
  const id = `node-${Date.now().toString(36)}`;
  editingNode.value = {
    isNew: true,
    node: {
      id,
      name: '',
      baseUrl: 'https://',
      authMode: 'authToken',
      authToken: '',
      models: {
        opus: 'claude-3-opus-20240229',
        sonnet: 'claude-3-7-sonnet-20250219',
        haiku: 'claude-3-5-haiku-20241022',
        defaultModel: 'claude-3-7-sonnet-20250219',
      },
    },
  };
  showNodeModal.value = true;
}

function openEditNode(node: RelayNode) {
  editingNode.value = {
    isNew: false,
    node: JSON.parse(JSON.stringify(node)),
  };
  showNodeModal.value = true;
}

async function handleSaveNode() {
  const n = editingNode.value.node;
  if (!n.name.trim() || !n.baseUrl.trim()) {
    notify('请填写中转站名称与 Base URL', 'warning');
    return;
  }
  try {
    const saved = await api.saveNode(n, true);
    const idx = nodes.value.findIndex((item) => item.id === saved.id);
    if (idx >= 0) {
      nodes.value[idx] = saved;
    } else {
      nodes.value.push(saved);
      if (currentGroup.value && !currentGroup.value.activeNodeId) {
        await handleSwitchNode(saved.id);
      }
    }
    showNodeModal.value = false;
    notify(`节点「${saved.name}」已保存`, 'success');
  } catch (err: any) {
    notify(`保存失败: ${err}`, 'error');
  }
}

async function handleDeleteNode(nodeId: string) {
  if (confirm('确定删除此中转节点吗？')) {
    try {
      await api.deleteNode(nodeId);
      nodes.value = nodes.value.filter((n) => n.id !== nodeId);
      if (currentGroup.value && currentGroup.value.activeNodeId === nodeId) {
        currentGroup.value.activeNodeId = null;
      }
      notify('节点已删除', 'info');
    } catch (err: any) {
      notify(`删除失败: ${err}`, 'error');
    }
  }
}

// Test Nodes
async function handleTestAllNodes() {
  if (nodes.value.length === 0) {
    notify('暂无节点可测试，请先添加节点', 'warning');
    return;
  }
  if (isTestingAll.value) return;
  isTestingAll.value = true;

  for (const n of nodes.value) {
    nodeTestingState.value[n.id] = { testing: true };
  }

  try {
    const results = await api.testAllNodes(false);
    for (const res of results) {
      nodeTestingState.value[res.nodeId] = { testing: false, result: res };
    }
    notify('节点测速完成', 'success');
  } catch (err: any) {
    notify(`测速异常: ${err}`, 'error');
  } finally {
    isTestingAll.value = false;
  }
}

// Copy Launch Command
async function handleCopyCommand() {
  if (!currentGroup.value) {
    notify('请先创建并选择一个模型分组', 'warning');
    return;
  }
  const cmd = previewCommand.value;
  try {
    await navigator.clipboard.writeText(cmd);
    copiedTooltip.value = true;
    notify('已复制启动命令到剪贴板！', 'success', 2000);
    setTimeout(() => {
      copiedTooltip.value = false;
    }, 2000);
  } catch {
    const textarea = document.createElement('textarea');
    textarea.value = cmd;
    document.body.appendChild(textarea);
    textarea.select();
    document.execCommand('copy');
    document.body.removeChild(textarea);
    copiedTooltip.value = true;
    notify('已复制启动命令到剪贴板！', 'success', 2000);
    setTimeout(() => {
      copiedTooltip.value = false;
    }, 2000);
  }
}

async function handleOpenConfigDir() {
  if (configDirInfo.value?.resolvedPath) {
    try {
      await api.openInExplorer(configDirInfo.value.resolvedPath);
    } catch (err: any) {
      notify(`打开目录失败: ${err}`, 'error');
    }
  }
}

async function handleViewSettingsJson() {
  if (!currentGroup.value) return;
  try {
    const content = await api.getGroupSettingsContent(currentGroup.value.id);
    currentJsonContent.value = content;
    currentEditingFilename.value = currentGroup.value.settingsFile;
    showJsonModal.value = true;
  } catch (err: any) {
    notify(`读取设置失败: ${err}`, 'error');
  }
}

async function handleSaveSettingsJson() {
  if (!currentGroup.value) return;
  try {
    await api.saveGroupSettingsContent(currentGroup.value.id, currentJsonContent.value);
    showJsonModal.value = false;
    notify('配置保存成功', 'success');
  } catch (err: any) {
    notify(`保存失败: ${err}`, 'error');
  }
}

// Settings Page Actions
async function handleSelectCustomDir() {
  try {
    const selected = await api.selectFolder(customDirInput.value || undefined);
    if (selected) {
      customDirInput.value = selected;
      preferences.value.customClaudeConfigDir = selected;
      preferences.value.configDirMode = 'custom';
      await api.savePreferences(preferences.value);
      configDirInfo.value = await api.getClaudeConfigDirInfo();
      configFiles.value = await api.listConfigFiles().catch(() => []);
      notify('已更新自定义配置目录', 'success');
    }
  } catch (err: any) {
    notify(`选择目录失败: ${err}`, 'error');
  }
}

async function setConfigDirMode(mode: 'auto' | 'custom') {
  preferences.value.configDirMode = mode;
  if (mode === 'custom' && !preferences.value.customClaudeConfigDir) {
    await handleSelectCustomDir();
  } else {
    await api.savePreferences(preferences.value);
    configDirInfo.value = await api.getClaudeConfigDirInfo();
    configFiles.value = await api.listConfigFiles().catch(() => []);
    notify(mode === 'auto' ? '已切换至自动检测' : '已切换至自定义配置目录', 'info');
  }
}

function openFilePreview(file: ConfigFileItem) {
  const group = groups.value.find((g) => g.settingsFile === file.name);
  if (group) {
    currentGroupId.value = group.id;
    handleViewSettingsJson();
  } else {
    api.openFileInEditor(file.path).catch(() => {
      api.openInExplorer(file.path);
    });
  }
}
</script>

<template>
  <div class="app-shell">
    <div class="app-container">
      <!-- ==================== VIEW 1: MAIN LAUNCHER ==================== -->
      <template v-if="currentView === 'main'">
        <!-- HEADER -->
        <header class="app-header">
          <div class="header-left">
            <img src="/logo.png" class="app-header-logo" alt="CC 多实例配置器 Logo" />
            <h1 class="app-title">CC 多实例配置器</h1>
          </div>

          <div class="header-right">
            <span class="badge-ready" title="基础配置目录正常">配置就绪</span>
            <button class="icon-gear-btn" @click="currentView = 'settings'" title="进入配置器设置页面">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="3"></circle>
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
              </svg>
            </button>
          </div>
        </header>

        <!-- TOAST BANNER -->
        <div v-if="statusBanner" class="status-toast" :class="statusBanner.type">
          <span>{{ statusBanner.text }}</span>
          <button class="toast-close" @click="statusBanner = null">×</button>
        </div>

        <!-- MAIN SCROLLABLE CONTENT AREA -->
        <main class="main-content">
          <!-- SECTION 1: MODEL GROUP SELECTOR -->
          <section class="block-section">
            <div class="block-label-row">
              <span class="block-label">当前模型分组 / 配置文件</span>
              <button
                v-if="currentGroup"
                class="btn-text-danger"
                @click="handleDeleteCurrentGroup"
                title="删除当前选中的分组"
              >
                删除分组
              </button>
            </div>

            <div class="group-select-row">
              <div class="select-container">
                <select
                  v-if="groups.length > 0"
                  class="styled-select"
                  v-model="currentGroupId"
                >
                  <option v-for="g in groups" :key="g.id" :value="g.id">
                    {{ g.name }}
                  </option>
                </select>
                <div v-else class="select-empty-box" @click="openAddGroup">
                  暂无模型分组，点击右侧「+」新建分组
                </div>
                <svg class="chevron-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="6 9 12 15 18 9"></polyline>
                </svg>
              </div>

              <!-- Circular Add Button -->
              <button class="btn-circle-add" @click="openAddGroup" title="新建模型分组">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <line x1="12" y1="5" x2="12" y2="19"></line>
                  <line x1="5" y1="12" x2="19" y2="12"></line>
                </svg>
              </button>
            </div>

            <div class="settings-filename-sub">
              <span
                v-if="currentGroup"
                class="file-link"
                @click="handleViewSettingsJson"
                title="点击查看 / 编辑此分组 JSON 配置"
              >
                {{ currentGroup.settingsFile }}
              </span>
              <span v-else class="file-link-disabled">my-&lt;slug&gt;.json</span>
            </div>
          </section>

          <!-- SECTION 2: RELAY NODES -->
          <section class="block-section">
            <div class="block-header-row">
              <h2 class="block-title">中转节点</h2>
              <span class="block-sub">选择后更新当前分组配置</span>
            </div>

            <!-- Nodes Cards List -->
            <div class="nodes-list-box">
              <div v-if="nodes.length === 0" class="empty-state-card">
                <p class="empty-title">暂无中转节点</p>
                <p class="empty-desc">点击下方「+ 添加节点」添加你的第一个中转 API 节点</p>
              </div>

              <div
                v-for="node in nodes"
                :key="node.id"
                class="node-item-card"
                :class="{ active: currentGroup?.activeNodeId === node.id }"
                @click="handleSwitchNode(node.id)"
              >
                <!-- Radio Status Circle -->
                <div class="node-left-icon">
                  <div v-if="currentGroup?.activeNodeId === node.id" class="radio-check-circle">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                      <polyline points="20 6 9 17 4 12"></polyline>
                    </svg>
                  </div>
                  <div v-else class="radio-empty-circle"></div>
                </div>

                <!-- Info -->
                <div class="node-info-col">
                  <div class="node-title">{{ node.name }}</div>
                  <div class="node-suburl">{{ node.baseUrl }}</div>
                </div>

                <!-- Right Meta -->
                <div class="node-meta-col">
                  <div class="meta-tag-wrap">
                    <span v-if="currentGroup?.activeNodeId === node.id" class="badge-active-node">
                      当前节点
                    </span>
                    <span v-else class="text-switch">切换</span>
                  </div>
                  <div class="meta-speed-wrap">
                    <span v-if="nodeTestingState[node.id]?.testing" class="speed-testing">测速中...</span>
                    <span v-else-if="nodeTestingState[node.id]?.result?.latencyMs !== undefined" class="speed-ms">
                      {{ nodeTestingState[node.id].result?.latencyMs }}ms
                    </span>
                    <span v-else class="speed-none">—</span>
                  </div>
                </div>

                <!-- Hover Edit Icon -->
                <button
                  class="btn-card-edit"
                  @click.stop="openEditNode(node)"
                  title="编辑节点"
                >
                  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path>
                    <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path>
                  </svg>
                </button>
              </div>
            </div>

            <!-- Two Pill Action Buttons -->
            <div class="nodes-pill-row">
              <button class="btn-pill" @click="openAddNode">
                <span class="plus-icon">+</span>
                <span>添加节点</span>
              </button>
              <button class="btn-pill" :disabled="isTestingAll" @click="handleTestAllNodes">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
                  <polyline points="22 4 12 14.01 9 11.01"></polyline>
                </svg>
                <span>{{ isTestingAll ? '正在测试...' : '测试节点' }}</span>
              </button>
            </div>
          </section>

          <!-- SECTION 3: CONFIG PREVIEW & LAUNCH -->
          <section class="preview-section">
            <div class="preview-top-row">
              <h2 class="preview-heading">启动配置预览</h2>
              <div class="preview-badges-right">
                <button
                  class="badge-pill-os"
                  @click="toggleOsPreview"
                  :title="`当前系统: ${hostOs.os}，点击可切换为 ${isCurrentWindows ? 'macOS' : 'Windows'} 格式`"
                >
                  {{ isCurrentWindows ? 'Windows' : 'macOS' }}
                </button>
                <span class="badge-pill-gray">独立会话</span>
              </div>
            </div>

            <div class="preview-group-desc">
              当前分组：
              <span v-if="currentGroup">
                <strong>{{ currentGroup.name }}</strong> ·
                <strong>{{ activeNodeForCurrentGroup?.name || '未选择节点' }}</strong>
              </span>
              <span v-else class="text-gray-light">未选择分组</span>
            </div>

            <!-- Command Text Box dynamically adapted for Windows / macOS -->
            <div class="code-box-line">
              <code>{{ previewCommand }}</code>
            </div>

            <!-- Full Width Black Pill Button -->
            <button
              class="btn-black-pill"
              :class="{ copied: copiedTooltip }"
              @click="handleCopyCommand"
            >
              <svg v-if="!copiedTooltip" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
              </svg>
              <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                <polyline points="20 6 9 17 4 12"></polyline>
              </svg>
              <span>{{ copiedTooltip ? '已复制启动命令！' : '复制启动命令' }}</span>
            </button>

            <p class="bottom-disclaimer">
              以上是交互概念原型，节点为示例数据；实际配置读写和终端启动由桌面程序实现。
            </p>
          </section>
        </main>
      </template>

      <!-- ==================== VIEW 2: SETTINGS PAGE ==================== -->
      <template v-else>
        <!-- HEADER -->
        <header class="app-header">
          <div class="header-left">
            <img src="/logo.png" class="app-header-logo" alt="CC 多实例配置器 Logo" />
            <h1 class="app-title">CC 多实例配置器 • 设置</h1>
          </div>

          <div class="header-right">
            <span class="badge-concept">概念界面</span>
            <button class="btn-nav-back" @click="currentView = 'main'" title="返回配置器主界面">
              返回配置器
            </button>
          </div>
        </header>

        <!-- TOAST BANNER -->
        <div v-if="statusBanner" class="status-toast" :class="statusBanner.type">
          <span>{{ statusBanner.text }}</span>
          <button class="toast-close" @click="statusBanner = null">×</button>
        </div>

        <!-- SETTINGS SCROLLABLE BODY -->
        <main class="settings-body">
          <!-- OS PREVIEW -->
          <section class="settings-card-item">
            <div class="settings-sub-label">操作系统预览 (自动检测当前系统)</div>
            <div class="segmented-pill-wrap">
              <button
                class="segmented-btn"
                :class="{ active: selectedOsPreview === 'macOS' }"
                @click="selectedOsPreview = 'macOS'"
              >
                macOS
              </button>
              <button
                class="segmented-btn"
                :class="{ active: selectedOsPreview === 'Windows' }"
                @click="selectedOsPreview = 'Windows'"
              >
                Windows
              </button>
            </div>
          </section>

          <!-- CONFIG DIRECTORY -->
          <section class="settings-card-item">
            <div class="settings-item-title">Claude 配置目录</div>

            <div class="radio-vertical-list">
              <label class="radio-field">
                <input
                  type="radio"
                  name="configDirMode"
                  value="auto"
                  :checked="preferences.configDirMode === 'auto'"
                  @change="setConfigDirMode('auto')"
                />
                <span class="radio-label-text">自动检测（推荐）</span>
              </label>

              <label class="radio-field">
                <input
                  type="radio"
                  name="configDirMode"
                  value="custom"
                  :checked="preferences.configDirMode === 'custom'"
                  @change="setConfigDirMode('custom')"
                />
                <span class="radio-label-text">自定义配置目录</span>
                <button
                  v-if="preferences.configDirMode === 'custom'"
                  class="btn-browse-dir"
                  @click="handleSelectCustomDir"
                >
                  浏览选择
                </button>
              </label>
            </div>

            <!-- Path Callout Box -->
            <div class="dir-callout-card">
              <div class="dir-callout-top">
                <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
                </svg>
                <span>当前配置目录示意</span>
              </div>
              <div class="dir-callout-path" @click="handleOpenConfigDir" title="点击在系统资源管理器中打开">
                {{ settingsDirDisplay }}
              </div>
              <div class="dir-callout-footnote">
                实际应用会显示解析后的绝对路径
              </div>
            </div>
          </section>

          <!-- CONFIG FILES PREVIEW -->
          <section class="settings-card-item">
            <div class="section-divider-line"></div>
            <h2 class="settings-item-title">配置文件预览</h2>

            <div class="files-preview-box">
              <div
                v-for="file in configFiles"
                :key="file.name"
                class="file-row"
                @click="openFilePreview(file)"
                title="点击查看配置内容"
              >
                <!-- File Icon with code symbol -->
                <div class="file-icon-box">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                    <polyline points="14 2 14 8 20 8"></polyline>
                    <line x1="16" y1="13" x2="8" y2="13"></line>
                    <line x1="16" y1="17" x2="8" y2="17"></line>
                    <line x1="10" y1="9" x2="8" y2="9"></line>
                  </svg>
                </div>

                <div class="file-name-col">
                  {{ file.name }}
                </div>

                <div class="file-badge-col">
                  <span v-if="file.fileType === 'base'" class="badge-tag-base">
                    基础文件
                  </span>
                  <span v-else class="badge-tag-group">
                    分组文件
                  </span>
                </div>
              </div>
            </div>
          </section>
        </main>
      </template>
    </div>

    <!-- MODAL: CREATE GROUP -->
    <div v-if="showCreateGroupModal" class="modal-overlay" @click.self="showCreateGroupModal = false">
      <div class="modal-box">
        <div class="modal-head">
          <h3 class="modal-head-title">新建模型分组</h3>
          <button class="modal-head-close" @click="showCreateGroupModal = false">×</button>
        </div>
        <div class="modal-content">
          <div class="input-field-group">
            <label class="input-label">分组展示名称</label>
            <input
              type="text"
              class="styled-input"
              v-model="newGroupName"
              @input="onGroupNameChange"
              placeholder="例如：Opus 4.6 或 Gemini 3 Pro"
              autofocus
            />
          </div>

          <div class="input-field-group">
            <label class="input-label">配置文件名标识 (Slug)</label>
            <div class="slug-input-wrapper">
              <span class="slug-affix">my-</span>
              <input
                type="text"
                class="slug-core-input"
                v-model="newGroupSlug"
                placeholder="opus-4.6"
              />
              <span class="slug-affix">.json</span>
            </div>
            <span class="input-helper-text">
              将在 {{ isCurrentWindows ? '%USERPROFILE%\\.claude\\' : '~/.claude/' }} 下生成 my-{{ newGroupSlug || 'xxx' }}.json，绝不影响基础 settings.json
            </span>
          </div>

          <div class="input-field-group">
            <label class="input-label">初始关联节点 (可选)</label>
            <select class="styled-input" v-model="newGroupInitialNodeId">
              <option value="">暂不关联</option>
              <option v-for="n in nodes" :key="n.id" :value="n.id">
                {{ n.name }} ({{ n.baseUrl }})
              </option>
            </select>
          </div>
        </div>
        <div class="modal-actions">
          <button class="btn-modal-cancel" @click="showCreateGroupModal = false">取消</button>
          <button class="btn-modal-confirm" @click="handleCreateGroup">确认创建</button>
        </div>
      </div>
    </div>

    <!-- MODAL: ADD / EDIT NODE -->
    <div v-if="showNodeModal" class="modal-overlay" @click.self="showNodeModal = false">
      <div class="modal-box modal-lg">
        <div class="modal-head">
          <h3 class="modal-head-title">{{ editingNode.isNew ? '添加中转节点' : '编辑中转节点' }}</h3>
          <button class="modal-head-close" @click="showNodeModal = false">×</button>
        </div>
        <div class="modal-content">
          <div class="input-grid-2">
            <div class="input-field-group">
              <label class="input-label">中转站名称</label>
              <input
                type="text"
                class="styled-input"
                v-model="editingNode.node.name"
                placeholder="例如：中转站 A"
              />
            </div>
            <div class="input-field-group">
              <label class="input-label">认证模式</label>
              <select class="styled-input" v-model="editingNode.node.authMode">
                <option value="authToken">ANTHROPIC_AUTH_TOKEN (中转推荐)</option>
                <option value="apiKey">ANTHROPIC_API_KEY (API Key)</option>
              </select>
            </div>
          </div>

          <div class="input-field-group">
            <label class="input-label">API Base URL</label>
            <input
              type="text"
              class="styled-input"
              v-model="editingNode.node.baseUrl"
              placeholder="例如：https://api-a.example.com"
            />
          </div>

          <div class="input-field-group">
            <label class="input-label">Token / 密钥</label>
            <div class="password-input-wrapper">
              <input
                :type="showPassword ? 'text' : 'password'"
                class="styled-input"
                v-model="editingNode.node.authToken"
                placeholder="sk-..."
              />
              <button class="btn-eye-toggle" type="button" @click="showPassword = !showPassword">
                {{ showPassword ? '隐藏' : '显示' }}
              </button>
            </div>
          </div>

          <div class="modal-divider-text">
            <span>模型映射 (覆盖默认模型)</span>
          </div>

          <div class="input-grid-2">
            <div class="input-field-group">
              <label class="input-label">默认模型 (ANTHROPIC_MODEL)</label>
              <input
                type="text"
                class="styled-input"
                v-model="editingNode.node.models.defaultModel"
                placeholder="claude-3-7-sonnet-20250219"
              />
            </div>
            <div class="input-field-group">
              <label class="input-label">Opus 映射模型</label>
              <input
                type="text"
                class="styled-input"
                v-model="editingNode.node.models.opus"
                placeholder="claude-3-opus-20240229"
              />
            </div>
          </div>

          <div class="input-grid-2">
            <div class="input-field-group">
              <label class="input-label">Sonnet 映射模型</label>
              <input
                type="text"
                class="styled-input"
                v-model="editingNode.node.models.sonnet"
                placeholder="claude-3-7-sonnet-20250219"
              />
            </div>
            <div class="input-field-group">
              <label class="input-label">Haiku 映射模型</label>
              <input
                type="text"
                class="styled-input"
                v-model="editingNode.node.models.haiku"
                placeholder="claude-3-5-haiku-20241022"
              />
            </div>
          </div>
        </div>
        <div class="modal-actions">
          <button
            v-if="!editingNode.isNew"
            class="btn-delete-node"
            @click="handleDeleteNode(editingNode.node.id)"
          >
            删除此节点
          </button>
          <div class="actions-right">
            <button class="btn-modal-cancel" @click="showNodeModal = false">取消</button>
            <button class="btn-modal-confirm" @click="handleSaveNode">保存节点</button>
          </div>
        </div>
      </div>
    </div>

    <!-- MODAL: VIEW / EDIT JSON -->
    <div v-if="showJsonModal" class="modal-overlay" @click.self="showJsonModal = false">
      <div class="modal-box modal-lg">
        <div class="modal-head">
          <h3 class="modal-head-title">查看配置 JSON ({{ currentEditingFilename }})</h3>
          <button class="modal-head-close" @click="showJsonModal = false">×</button>
        </div>
        <div class="modal-content">
          <p class="input-helper-text" style="margin-bottom: 8px;">
            此文件独立保存在 Claude 配置目录，仅当前分组启动时通过 <code>--settings</code> 加载。
          </p>
          <textarea class="json-editor-area" v-model="currentJsonContent" rows="14"></textarea>
        </div>
        <div class="modal-actions">
          <button class="btn-modal-cancel" @click="showJsonModal = false">取消</button>
          <button class="btn-modal-confirm" @click="handleSaveSettingsJson">保存</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* App Shell & Native Container */
.app-shell {
  width: 100%;
  height: 100%;
  background-color: #ffffff;
  overflow: hidden;
  box-sizing: border-box;
}

.app-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 20px 24px 18px 24px;
  box-sizing: border-box;
  overflow: hidden;
}

/* Header */
.app-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-bottom: 14px;
  flex-shrink: 0;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.app-header-logo {
  width: 26px;
  height: 26px;
  border-radius: 7px;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.12);
  object-fit: cover;
  flex-shrink: 0;
}

.app-icon {
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #0f172a;
}

.app-title {
  margin: 0;
  font-size: 19px;
  font-weight: 700;
  color: #0f172a;
  letter-spacing: -0.01em;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.badge-ready {
  background: #e6f9ed;
  color: #16a34a;
  border: 1px solid #bbf7d0;
  font-size: 12px;
  font-weight: 500;
  padding: 3px 10px;
  border-radius: 9999px;
}

.badge-concept {
  background: #f1f5f9;
  color: #475569;
  font-size: 11px;
  font-weight: 500;
  padding: 3px 9px;
  border-radius: 6px;
}

.icon-gear-btn {
  background: transparent;
  border: none;
  color: #94a3b8;
  cursor: pointer;
  padding: 5px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  transition: all 0.15s;
}

.icon-gear-btn:hover {
  color: #0f172a;
  background: #f1f5f9;
}

.btn-nav-back {
  background: #f8fafc;
  border: 1px solid #cbd5e1;
  color: #334155;
  font-size: 12px;
  font-weight: 500;
  padding: 4px 10px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
}
.btn-nav-back:hover {
  background: #0f172a;
  color: #fff;
  border-color: #0f172a;
}

/* Toast */
.status-toast {
  padding: 7px 12px;
  border-radius: 8px;
  font-size: 13px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  flex-shrink: 0;
}
.status-toast.success { background: #dcfce7; color: #15803d; }
.status-toast.warning { background: #fef9c3; color: #a16207; }
.status-toast.error { background: #fee2e2; color: #b91c1c; }
.status-toast.info { background: #e0f2fe; color: #0369a1; }
.toast-close { background: none; border: none; font-size: 16px; cursor: pointer; color: inherit; }

/* Main Scrollable Area */
.main-content {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding-right: 2px;
}

/* Block Section */
.block-section {
  display: flex;
  flex-direction: column;
}

.block-label-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 7px;
}

.block-label {
  font-size: 13px;
  color: #64748b;
  font-weight: 500;
}

.btn-text-danger {
  background: none;
  border: none;
  color: #ef4444;
  font-size: 12px;
  cursor: pointer;
  padding: 0;
}
.btn-text-danger:hover {
  text-decoration: underline;
}

.group-select-row {
  display: flex;
  gap: 10px;
  align-items: center;
}

.select-container {
  flex: 1;
  position: relative;
  display: flex;
  align-items: center;
}

.styled-select {
  width: 100%;
  appearance: none;
  background: #ffffff;
  border: 1px solid #cbd5e1;
  border-radius: 10px;
  padding: 9px 36px 9px 14px;
  font-size: 15px;
  font-weight: 500;
  color: #0f172a;
  outline: none;
  cursor: pointer;
  transition: border-color 0.15s;
}

.styled-select:focus {
  border-color: #0f172a;
}

.select-empty-box {
  width: 100%;
  border: 1px dashed #cbd5e1;
  border-radius: 10px;
  padding: 9px 14px;
  font-size: 14px;
  color: #94a3b8;
  cursor: pointer;
}

.chevron-icon {
  position: absolute;
  right: 14px;
  pointer-events: none;
  color: #64748b;
}

.btn-circle-add {
  width: 42px;
  height: 42px;
  border: 1px solid #cbd5e1;
  border-radius: 12px;
  background: #ffffff;
  color: #334155;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.15s;
}

.btn-circle-add:hover {
  background: #f8fafc;
  border-color: #94a3b8;
  color: #0f172a;
}

.settings-filename-sub {
  margin-top: 5px;
  font-size: 13px;
  color: #64748b;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.file-link {
  cursor: pointer;
}
.file-link:hover {
  text-decoration: underline;
  color: #0f172a;
}

.file-link-disabled {
  color: #94a3b8;
}

/* Middle Nodes Section */
.block-header-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 10px;
}

.block-title {
  margin: 0;
  font-size: 15px;
  font-weight: 700;
  color: #0f172a;
}

.block-sub {
  font-size: 12px;
  color: #64748b;
}

.nodes-list-box {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 250px;
  overflow-y: auto;
  overflow-x: hidden;
  padding-right: 2px;
}

.empty-state-card {
  padding: 20px;
  border: 1px dashed #cbd5e1;
  border-radius: 12px;
  text-align: center;
  color: #64748b;
}

.empty-title {
  margin: 0 0 4px 0;
  font-weight: 500;
  font-size: 14px;
  color: #334155;
}

.empty-desc {
  margin: 0;
  font-size: 12px;
  color: #94a3b8;
}

/* Node item card */
.node-item-card {
  position: relative;
  display: flex;
  align-items: center;
  padding: 12px 16px;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  background: #ffffff;
  cursor: pointer;
  transition: all 0.15s;
}

.node-item-card:hover {
  border-color: #cbd5e1;
  background: #fafafa;
}

.node-item-card.active {
  border: 1.5px solid #10b981;
  background: #ffffff;
}

.node-left-icon {
  margin-right: 14px;
  display: flex;
  align-items: center;
}

.radio-check-circle {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: #10b981;
  color: #ffffff;
  display: flex;
  align-items: center;
  justify-content: center;
}

.radio-empty-circle {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #94a3b8;
}

.node-info-col {
  flex: 1;
}

.node-title {
  font-size: 15px;
  font-weight: 600;
  color: #0f172a;
}

.node-suburl {
  font-size: 13px;
  color: #64748b;
  margin-top: 1px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.node-meta-col {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 3px;
  margin-left: 12px;
}

.badge-active-node {
  background: #dcfce7;
  color: #15803d;
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 6px;
}

.text-switch {
  font-size: 13px;
  color: #64748b;
}

.node-item-card:hover .text-switch {
  color: #0f172a;
}

.speed-ms {
  font-size: 13px;
  color: #64748b;
}

.speed-testing {
  font-size: 12px;
  color: #3b82f6;
}

.speed-none {
  font-size: 13px;
  color: #94a3b8;
}

.btn-card-edit {
  position: absolute;
  top: 8px;
  right: 8px;
  background: transparent;
  border: none;
  color: #cbd5e1;
  padding: 4px;
  border-radius: 4px;
  cursor: pointer;
  display: none;
}

.node-item-card:hover .btn-card-edit {
  display: block;
}

.btn-card-edit:hover {
  color: #0f172a;
  background: #f1f5f9;
}

/* Action Pill Buttons below nodes */
.nodes-pill-row {
  display: flex;
  gap: 12px;
  margin-top: 10px;
}

.btn-pill {
  flex: 1;
  border: 1px solid #cbd5e1;
  border-radius: 9999px;
  background: #ffffff;
  color: #334155;
  font-size: 13px;
  font-weight: 500;
  padding: 9px 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-pill:hover:not(:disabled) {
  border-color: #94a3b8;
  background: #f8fafc;
  color: #0f172a;
}

.btn-pill:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.plus-icon {
  font-size: 16px;
}

/* SECTION 3: LAUNCH CONFIG PREVIEW */
.preview-section {
  border-top: 1px solid #f1f5f9;
  padding-top: 16px;
  display: flex;
  flex-direction: column;
}

.preview-top-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.preview-heading {
  margin: 0;
  font-size: 15px;
  font-weight: 700;
  color: #0f172a;
}

.preview-badges-right {
  display: flex;
  align-items: center;
  gap: 6px;
}

.badge-pill-os {
  background: #e0f2fe;
  color: #0369a1;
  border: 1px solid #bae6fd;
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
}
.badge-pill-os:hover {
  background: #0284c7;
  color: #ffffff;
  border-color: #0284c7;
}

.badge-pill-gray {
  background: #f1f5f9;
  color: #475569;
  font-size: 11px;
  font-weight: 500;
  padding: 3px 8px;
  border-radius: 6px;
}

.preview-group-desc {
  font-size: 13px;
  color: #475569;
  margin-top: 3px;
}

.preview-group-desc strong {
  color: #0f172a;
}

.text-gray-light {
  color: #94a3b8;
}

.code-box-line {
  margin: 14px 0 12px 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 13px;
  color: #475569;
  user-select: all;
  word-break: break-all;
}

.btn-black-pill {
  width: 100%;
  background: #0f172a;
  color: #ffffff;
  border: none;
  border-radius: 9999px;
  padding: 12px 20px;
  font-size: 14px;
  font-weight: 500;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-black-pill:hover {
  background: #1e293b;
}

.btn-black-pill.copied {
  background: #16a34a;
}

.bottom-disclaimer {
  font-size: 12px;
  color: #94a3b8;
  text-align: center;
  margin: 12px 0 0 0;
  line-height: 1.4;
}

/* ==================== SETTINGS PAGE STYLES ==================== */
.settings-body {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding-right: 2px;
}

.settings-card-item {
  display: flex;
  flex-direction: column;
}

.settings-sub-label {
  font-size: 13px;
  color: #64748b;
  margin-bottom: 7px;
}

.settings-item-title {
  font-size: 14px;
  font-weight: 600;
  color: #0f172a;
  margin: 0 0 9px 0;
}

/* Segmented Control */
.segmented-pill-wrap {
  display: flex;
  background: #f1f5f9;
  padding: 3px;
  border-radius: 9999px;
  gap: 2px;
}

.segmented-btn {
  flex: 1;
  padding: 7px 16px;
  border-radius: 9999px;
  border: none;
  background: transparent;
  color: #64748b;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.segmented-btn.active {
  background: #ffffff;
  color: #0f172a;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

/* Radios */
.radio-vertical-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 12px;
}

.radio-field {
  display: flex;
  align-items: center;
  gap: 9px;
  cursor: pointer;
}

.radio-field input[type="radio"] {
  accent-color: #0f172a;
  width: 16px;
  height: 16px;
  cursor: pointer;
}

.radio-label-text {
  font-size: 14px;
  color: #1e293b;
  font-weight: 500;
}

.btn-browse-dir {
  margin-left: auto;
  border: 1px solid #cbd5e1;
  background: #ffffff;
  color: #334155;
  font-size: 12px;
  padding: 3px 9px;
  border-radius: 6px;
  cursor: pointer;
}
.btn-browse-dir:hover {
  background: #f8fafc;
  color: #0f172a;
}

/* Dir Callout Card */
.dir-callout-card {
  background: #f8fafc;
  border: 1px solid #f1f5f9;
  border-radius: 10px;
  padding: 12px 14px;
}

.dir-callout-top {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #475569;
  font-size: 13px;
  font-weight: 500;
  margin-bottom: 4px;
}

.dir-callout-path {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 13px;
  color: #0f172a;
  margin-bottom: 3px;
  cursor: pointer;
  word-break: break-all;
}
.dir-callout-path:hover {
  text-decoration: underline;
}

.dir-callout-footnote {
  font-size: 11px;
  color: #94a3b8;
}

.section-divider-line {
  border-top: 1px solid #f1f5f9;
  margin-bottom: 14px;
}

/* Files Preview List */
.files-preview-box {
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  overflow-y: auto;
  overflow-x: hidden;
  max-height: 240px;
  background: #ffffff;
}

.file-row {
  display: flex;
  align-items: center;
  padding: 11px 16px;
  border-bottom: 1px solid #f1f5f9;
  cursor: pointer;
  transition: background 0.15s;
}
.file-row:last-child {
  border-bottom: none;
}
.file-row:hover {
  background: #f8fafc;
}

.file-icon-box {
  margin-right: 12px;
  color: #64748b;
  display: flex;
  align-items: center;
}

.file-name-col {
  flex: 1;
  font-size: 14px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  color: #1e293b;
}

.file-badge-col {
  display: flex;
  align-items: center;
}

.badge-tag-base {
  background: #f1f5f9;
  color: #475569;
  font-size: 11px;
  font-weight: 500;
  padding: 3px 8px;
  border-radius: 6px;
}

.badge-tag-group {
  background: #e6f9ed;
  color: #16a34a;
  font-size: 11px;
  font-weight: 500;
  padding: 3px 8px;
  border-radius: 6px;
}

/* MODALS */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(15, 23, 42, 0.4);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-box {
  background: #ffffff;
  border-radius: 14px;
  width: 440px;
  max-width: 90vw;
  box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
  display: flex;
  flex-direction: column;
}

.modal-lg {
  width: 520px;
}

.modal-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px 18px;
  border-bottom: 1px solid #f1f5f9;
}

.modal-head-title {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: #0f172a;
}

.modal-head-close {
  background: none;
  border: none;
  font-size: 20px;
  color: #94a3b8;
  cursor: pointer;
}

.modal-content {
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 13px;
  max-height: 70vh;
  overflow-y: auto;
}

.input-field-group {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.input-label {
  font-size: 13px;
  font-weight: 500;
  color: #334155;
}

.styled-input {
  width: 100%;
  border: 1px solid #cbd5e1;
  border-radius: 8px;
  padding: 8px 12px;
  font-size: 14px;
  color: #0f172a;
  outline: none;
  box-sizing: border-box;
}

.styled-input:focus {
  border-color: #0f172a;
}

.slug-input-wrapper {
  display: flex;
  align-items: center;
  border: 1px solid #cbd5e1;
  border-radius: 8px;
  overflow: hidden;
}

.slug-affix {
  background: #f8fafc;
  padding: 8px 10px;
  color: #64748b;
  font-size: 13px;
  font-family: monospace;
}

.slug-core-input {
  border: none;
  padding: 8px;
  font-family: monospace;
  font-size: 14px;
  flex: 1;
}

.slug-core-input:focus {
  outline: none;
}

.input-helper-text {
  font-size: 11px;
  color: #64748b;
}

.input-grid-2 {
  display: flex;
  gap: 12px;
}
.input-grid-2 .input-field-group {
  flex: 1;
}

.password-input-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}

.btn-eye-toggle {
  position: absolute;
  right: 10px;
  background: none;
  border: none;
  font-size: 12px;
  color: #64748b;
  cursor: pointer;
}

.modal-divider-text {
  border-bottom: 1px solid #f1f5f9;
  line-height: 0.1em;
  margin: 8px 0;
  text-align: center;
}
.modal-divider-text span {
  background: #fff;
  padding: 0 10px;
  color: #94a3b8;
  font-size: 12px;
}

.json-editor-area {
  width: 100%;
  border: 1px solid #cbd5e1;
  border-radius: 8px;
  font-family: monospace;
  font-size: 12px;
  padding: 10px;
  box-sizing: border-box;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 10px;
  padding: 12px 18px;
  border-top: 1px solid #f1f5f9;
}

.actions-right {
  display: flex;
  gap: 10px;
}

.btn-modal-cancel {
  border: 1px solid #cbd5e1;
  background: #ffffff;
  color: #475569;
  padding: 6px 14px;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
}

.btn-modal-confirm {
  border: none;
  background: #0f172a;
  color: #ffffff;
  padding: 6px 16px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
}

.btn-delete-node {
  margin-right: auto;
  background: none;
  border: none;
  color: #ef4444;
  font-size: 13px;
  cursor: pointer;
  padding: 0;
}
</style>
