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

// Settings view specific state
const selectedOsPreview = ref<'macOS' | 'Windows'>('macOS');
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

// Short display path e.g. ~/.claude/my-opus-4.6.json
const displaySettingsPath = computed(() => {
  if (!currentGroup.value) return '~/.claude/my-custom.json';
  return `~/.claude/${currentGroup.value.settingsFile}`;
});

// Command preview
const previewCommand = computed(() => {
  if (!currentGroup.value) {
    return 'claude --settings ~/.claude/my-config.json';
  }
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
    // Windows default representation
    return configDirInfo.value?.resolvedPath || '%USERPROFILE%\\.claude\\';
  }
});

function notify(text: string, type: 'success' | 'warning' | 'error' | 'info' = 'info', ms = 3500) {
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
    const [cfg, allNodes, dirInfo, binaryInfo, files] = await Promise.all([
      api.getLauncherConfig(),
      api.getNodes(),
      api.getClaudeConfigDirInfo(),
      api.detectClaudeBinary(),
      api.listConfigFiles().catch(() => []),
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
    // Refresh config files list
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
    notify(`已切换至节点「${nodes.value.find((n) => n.id === nodeId)?.name}」并更新配置`, 'success');
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
  // If it's a group file, we can open in JSON viewer
  const group = groups.value.find((g) => g.settingsFile === file.name);
  if (group) {
    currentGroupId.value = group.id;
    handleViewSettingsJson();
  } else {
    // Open in Explorer or default editor
    api.openFileInEditor(file.path).catch(() => {
      api.openInExplorer(file.path);
    });
  }
}
</script>

<template>
  <div class="window-wrapper">
    <!-- ==================== VIEW 1: MAIN LAUNCHER ==================== -->
    <div v-if="currentView === 'main'" class="launcher-card">
      <!-- HEADER -->
      <header class="card-header">
        <div class="header-title-box">
          <div class="app-icon">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="3" width="18" height="18" rx="4" />
              <polyline points="7 10 10 13 7 16" />
              <line x1="12" y1="16" x2="16" y2="16" />
            </svg>
          </div>
          <h1 class="app-title">Claude Launcher</h1>
        </div>

        <div class="header-badges">
          <span class="status-badge ready" title="基础配置目录正常">配置就绪</span>
          <button class="settings-gear-btn" @click="currentView = 'settings'" title="进入启动器设置页面">
            <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="3"></circle>
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
            </svg>
          </button>
        </div>
      </header>

      <!-- ALERT BANNER -->
      <div v-if="statusBanner" class="status-toast" :class="statusBanner.type">
        <span>{{ statusBanner.text }}</span>
        <button class="toast-close" @click="statusBanner = null">×</button>
      </div>

      <!-- SECTION 1: MODEL GROUP SELECTOR -->
      <section class="section-group">
        <div class="label-row">
          <label class="section-label">当前模型分组 / 配置文件</label>
          <button
            v-if="currentGroup"
            class="group-delete-link"
            @click="handleDeleteCurrentGroup"
            title="删除当前选中的分组"
          >
            删除此分组
          </button>
        </div>

        <div class="group-select-row">
          <div class="custom-select-wrap">
            <select
              v-if="groups.length > 0"
              class="group-select"
              v-model="currentGroupId"
            >
              <option v-for="g in groups" :key="g.id" :value="g.id">
                {{ g.name }}
              </option>
            </select>
            <div v-else class="empty-select-placeholder" @click="openAddGroup">
              暂无模型分组，点击右侧「+」新建分组
            </div>
            <svg class="select-chevron" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="6 9 12 15 18 9"></polyline>
            </svg>
          </div>

          <!-- Add Group Button -->
          <button class="add-group-btn" @click="openAddGroup" title="新建模型分组">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="12" y1="5" x2="12" y2="19"></line>
              <line x1="5" y1="12" x2="19" y2="12"></line>
            </svg>
          </button>
        </div>

        <!-- Settings Filename subtitle -->
        <div class="settings-file-text">
          <span
            v-if="currentGroup"
            class="clickable-file"
            @click="handleViewSettingsJson"
            title="点击查看 / 编辑此分组 JSON 配置"
          >
            {{ currentGroup.settingsFile }}
          </span>
          <span v-else class="empty-file-hint">my-&lt;slug&gt;.json</span>
        </div>
      </section>

      <!-- SECTION 2: RELAY NODES -->
      <section class="section-nodes">
        <div class="nodes-header-row">
          <h2 class="nodes-title">中转节点</h2>
          <span class="nodes-desc">选择后更新当前分组配置</span>
        </div>

        <!-- Node Cards List -->
        <div class="nodes-container">
          <div v-if="nodes.length === 0" class="empty-nodes-box">
            <p>暂无中转节点</p>
            <span class="empty-sub">点击下方「+ 添加节点」添加你的第一个中转 API 节点</span>
          </div>

          <div
            v-for="node in nodes"
            :key="node.id"
            class="node-card"
            :class="{ active: currentGroup?.activeNodeId === node.id }"
            @click="handleSwitchNode(node.id)"
          >
            <!-- Radio / Check Status Icon -->
            <div class="node-icon-box">
              <div v-if="currentGroup?.activeNodeId === node.id" class="check-circle">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                  <polyline points="20 6 9 17 4 12"></polyline>
                </svg>
              </div>
              <div v-else class="empty-circle"></div>
            </div>

            <!-- Node Info -->
            <div class="node-content">
              <div class="node-name-row">
                <span class="node-name">{{ node.name }}</span>
              </div>
              <div class="node-url">{{ node.baseUrl }}</div>
            </div>

            <!-- Right Status / Actions -->
            <div class="node-meta-box">
              <div class="meta-top">
                <span v-if="currentGroup?.activeNodeId === node.id" class="badge-current">
                  当前节点
                </span>
                <span v-else class="btn-switch-text">切换</span>
              </div>

              <div class="meta-bottom">
                <span v-if="nodeTestingState[node.id]?.testing" class="latency-testing">测速中...</span>
                <span v-else-if="nodeTestingState[node.id]?.result?.latencyMs !== undefined" class="latency-ms">
                  {{ nodeTestingState[node.id].result?.latencyMs }}ms
                </span>
                <span v-else class="latency-empty">—</span>
              </div>
            </div>

            <!-- Quick Edit Menu (Icon Button) -->
            <button
              class="node-edit-icon"
              @click.stop="openEditNode(node)"
              title="编辑此节点"
            >
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path>
                <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path>
              </svg>
            </button>
          </div>
        </div>

        <!-- Action Pill Buttons: Add Node & Test Nodes -->
        <div class="nodes-action-row">
          <button class="pill-btn" @click="openAddNode">
            <span class="btn-icon">+</span>
            <span>添加节点</span>
          </button>
          <button class="pill-btn" :disabled="isTestingAll" @click="handleTestAllNodes">
            <svg class="btn-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
              <polyline points="22 4 12 14.01 9 11.01"></polyline>
            </svg>
            <span>{{ isTestingAll ? '正在测试...' : '测试节点' }}</span>
          </button>
        </div>
      </section>

      <!-- SECTION 3: LAUNCH CONFIG PREVIEW -->
      <section class="section-preview">
        <div class="preview-header-row">
          <h2 class="preview-title">启动配置预览</h2>
          <span class="badge-session">独立会话</span>
        </div>

        <div class="preview-subtitle">
          当前分组：
          <span v-if="currentGroup">
            <strong>{{ currentGroup.name }}</strong> ·
            <strong>{{ activeNodeForCurrentGroup?.name || '未选择节点' }}</strong>
          </span>
          <span v-else class="text-muted">未选择分组</span>
        </div>

        <!-- Command Code Box -->
        <div class="command-box">
          <code>{{ previewCommand }}</code>
        </div>

        <!-- Big Black Copy Button -->
        <button
          class="btn-copy-command"
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

        <p class="footer-caption">
          以上是交互概念原型，节点为示例数据；实际配置读写和终端启动由桌面程序实现。
        </p>
      </section>
    </div>

    <!-- ==================== VIEW 2: SETTINGS PAGE ==================== -->
    <div v-else class="launcher-card settings-card">
      <!-- HEADER -->
      <header class="card-header">
        <div class="header-title-box">
          <div class="app-icon">
            <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="3"></circle>
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
            </svg>
          </div>
          <h1 class="app-title">Claude Launcher • 设置</h1>
        </div>

        <div class="header-badges">
          <span class="badge-concept">概念界面</span>
          <button class="btn-back-link" @click="currentView = 'main'" title="返回启动器主界面">
            返回启动器
          </button>
        </div>
      </header>

      <!-- ALERT BANNER IN SETTINGS -->
      <div v-if="statusBanner" class="status-toast" :class="statusBanner.type">
        <span>{{ statusBanner.text }}</span>
        <button class="toast-close" @click="statusBanner = null">×</button>
      </div>

      <!-- SECTION 1: OS PREVIEW SEGMENTED CONTROL -->
      <section class="settings-group">
        <label class="settings-label">操作系统预览</label>
        <div class="os-segmented-control">
          <button
            class="os-segment"
            :class="{ active: selectedOsPreview === 'macOS' }"
            @click="selectedOsPreview = 'macOS'"
          >
            macOS
          </button>
          <button
            class="os-segment"
            :class="{ active: selectedOsPreview === 'Windows' }"
            @click="selectedOsPreview = 'Windows'"
          >
            Windows
          </button>
        </div>
      </section>

      <!-- SECTION 2: CLAUDE CONFIG DIR -->
      <section class="settings-group">
        <label class="settings-label-strong">Claude 配置目录</label>

        <!-- Radio Options -->
        <div class="radio-options-column">
          <label class="custom-radio-row">
            <input
              type="radio"
              name="configDirMode"
              value="auto"
              :checked="preferences.configDirMode === 'auto'"
              @change="setConfigDirMode('auto')"
            />
            <span class="radio-text">自动检测（推荐）</span>
          </label>

          <label class="custom-radio-row">
            <input
              type="radio"
              name="configDirMode"
              value="custom"
              :checked="preferences.configDirMode === 'custom'"
              @change="setConfigDirMode('custom')"
            />
            <span class="radio-text">自定义配置目录</span>
            <button
              v-if="preferences.configDirMode === 'custom'"
              class="btn-select-dir"
              @click="handleSelectCustomDir"
            >
              浏览选择
            </button>
          </label>
        </div>

        <!-- Directory Preview Callout Box -->
        <div class="dir-preview-box">
          <div class="dir-preview-header">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
            </svg>
            <span class="dir-header-title">当前配置目录示意</span>
          </div>
          <div class="dir-path-value" @click="handleOpenConfigDir" title="点击在系统资源管理器中打开">
            {{ settingsDirDisplay }}
          </div>
          <div class="dir-path-note">
            实际应用会显示解析后的绝对路径
          </div>
        </div>
      </section>

      <!-- SECTION 3: CONFIG FILES PREVIEW -->
      <section class="settings-group">
        <div class="section-divider-clean"></div>
        <h2 class="settings-section-title">配置文件预览</h2>

        <div class="files-preview-list">
          <div
            v-for="file in configFiles"
            :key="file.name"
            class="file-row-item"
            @click="openFilePreview(file)"
            title="点击查看此文件配置"
          >
            <!-- File Icon with {} -->
            <div class="file-icon-wrap">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                <polyline points="14 2 14 8 20 8"></polyline>
                <line x1="16" y1="13" x2="8" y2="13"></line>
                <line x1="16" y1="17" x2="8" y2="17"></line>
                <line x1="10" y1="9" x2="8" y2="9"></line>
              </svg>
            </div>

            <!-- File Name -->
            <div class="file-name-text">
              {{ file.name }}
            </div>

            <!-- Tag: 基础文件 / 分组文件 -->
            <div class="file-tag-box">
              <span
                v-if="file.fileType === 'base'"
                class="badge-file base"
              >
                基础文件
              </span>
              <span
                v-else-if="file.fileType === 'group'"
                class="badge-file group"
              >
                分组文件
              </span>
              <span
                v-else
                class="badge-file other"
              >
                配置文件
              </span>
            </div>
          </div>
        </div>
      </section>
    </div>

    <!-- MODAL: CREATE GROUP -->
    <div v-if="showCreateGroupModal" class="modal-backdrop" @click.self="showCreateGroupModal = false">
      <div class="modal-card">
        <div class="modal-header">
          <h3 class="modal-title">新建模型分组</h3>
          <button class="modal-close" @click="showCreateGroupModal = false">×</button>
        </div>
        <div class="modal-body">
          <div class="form-item">
            <label class="form-label">分组展示名称</label>
            <input
              type="text"
              class="form-input"
              v-model="newGroupName"
              @input="onGroupNameChange"
              placeholder="例如：Opus 4.6 或 Gemini 3 Pro"
              autofocus
            />
          </div>

          <div class="form-item">
            <label class="form-label">配置文件名标识 (Slug)</label>
            <div class="slug-row">
              <span class="slug-tag">my-</span>
              <input
                type="text"
                class="form-input slug-input"
                v-model="newGroupSlug"
                placeholder="opus-4.6"
              />
              <span class="slug-tag">.json</span>
            </div>
            <span class="form-hint">将在 ~/.claude/ 下生成 my-{{ newGroupSlug || 'xxx' }}.json，绝不影响基础 settings.json</span>
          </div>

          <div class="form-item">
            <label class="form-label">初始关联节点 (可选)</label>
            <select class="form-input" v-model="newGroupInitialNodeId">
              <option value="">暂不关联</option>
              <option v-for="n in nodes" :key="n.id" :value="n.id">
                {{ n.name }} ({{ n.baseUrl }})
              </option>
            </select>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn-cancel" @click="showCreateGroupModal = false">取消</button>
          <button class="btn-confirm" @click="handleCreateGroup">确认创建</button>
        </div>
      </div>
    </div>

    <!-- MODAL: ADD / EDIT NODE -->
    <div v-if="showNodeModal" class="modal-backdrop" @click.self="showNodeModal = false">
      <div class="modal-card modal-wide">
        <div class="modal-header">
          <h3 class="modal-title">{{ editingNode.isNew ? '添加中转节点' : '编辑中转节点' }}</h3>
          <button class="modal-close" @click="showNodeModal = false">×</button>
        </div>
        <div class="modal-body">
          <div class="form-grid">
            <div class="form-item">
              <label class="form-label">中转站名称</label>
              <input
                type="text"
                class="form-input"
                v-model="editingNode.node.name"
                placeholder="例如：中转站 A"
              />
            </div>
            <div class="form-item">
              <label class="form-label">认证模式</label>
              <select class="form-input" v-model="editingNode.node.authMode">
                <option value="authToken">ANTHROPIC_AUTH_TOKEN (中转推荐)</option>
                <option value="apiKey">ANTHROPIC_API_KEY (API Key)</option>
              </select>
            </div>
          </div>

          <div class="form-item">
            <label class="form-label">API Base URL</label>
            <input
              type="text"
              class="form-input"
              v-model="editingNode.node.baseUrl"
              placeholder="例如：https://api-a.example.com"
            />
          </div>

          <div class="form-item">
            <label class="form-label">Token / 密钥</label>
            <div class="password-wrap">
              <input
                :type="showPassword ? 'text' : 'password'"
                class="form-input"
                v-model="editingNode.node.authToken"
                placeholder="sk-..."
              />
              <button class="toggle-eye" type="button" @click="showPassword = !showPassword">
                {{ showPassword ? '隐藏' : '显示' }}
              </button>
            </div>
          </div>

          <div class="form-divider">
            <span>模型映射 (覆盖默认模型)</span>
          </div>

          <div class="form-grid">
            <div class="form-item">
              <label class="form-label">默认模型 (ANTHROPIC_MODEL)</label>
              <input
                type="text"
                class="form-input"
                v-model="editingNode.node.models.defaultModel"
                placeholder="claude-3-7-sonnet-20250219"
              />
            </div>
            <div class="form-item">
              <label class="form-label">Opus 映射模型</label>
              <input
                type="text"
                class="form-input"
                v-model="editingNode.node.models.opus"
                placeholder="claude-3-opus-20240229"
              />
            </div>
          </div>

          <div class="form-grid">
            <div class="form-item">
              <label class="form-label">Sonnet 映射模型</label>
              <input
                type="text"
                class="form-input"
                v-model="editingNode.node.models.sonnet"
                placeholder="claude-3-7-sonnet-20250219"
              />
            </div>
            <div class="form-item">
              <label class="form-label">Haiku 映射模型</label>
              <input
                type="text"
                class="form-input"
                v-model="editingNode.node.models.haiku"
                placeholder="claude-3-5-haiku-20241022"
              />
            </div>
          </div>
        </div>
        <div class="modal-footer">
          <button
            v-if="!editingNode.isNew"
            class="btn-danger-link"
            @click="handleDeleteNode(editingNode.node.id)"
          >
            删除此节点
          </button>
          <div class="footer-right">
            <button class="btn-cancel" @click="showNodeModal = false">取消</button>
            <button class="btn-confirm" @click="handleSaveNode">保存节点</button>
          </div>
        </div>
      </div>
    </div>

    <!-- MODAL: VIEW / EDIT JSON -->
    <div v-if="showJsonModal" class="modal-backdrop" @click.self="showJsonModal = false">
      <div class="modal-card modal-wide">
        <div class="modal-header">
          <h3 class="modal-title">查看配置 JSON ({{ currentEditingFilename }})</h3>
          <button class="modal-close" @click="showJsonModal = false">×</button>
        </div>
        <div class="modal-body">
          <p class="form-hint" style="margin-bottom: 8px;">
            此文件独立保存在 Claude 配置目录，仅当前分组启动时通过 <code>--settings</code> 加载。
          </p>
          <textarea class="json-textarea" v-model="currentJsonContent" rows="14"></textarea>
        </div>
        <div class="modal-footer">
          <button class="btn-cancel" @click="showJsonModal = false">取消</button>
          <button class="btn-confirm" @click="handleSaveSettingsJson">保存</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Reset & Layout */
.window-wrapper {
  min-height: 100vh;
  width: 100vw;
  background-color: #f8fafc;
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 24px 16px;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  color: #1e293b;
  box-sizing: border-box;
  user-select: none;
}

/* Card Container */
.launcher-card {
  width: 100%;
  max-width: 620px;
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 16px;
  padding: 24px 28px;
  box-shadow: 0 4px 25px -4px rgba(0, 0, 0, 0.05), 0 2px 10px -2px rgba(0, 0, 0, 0.02);
  display: flex;
  flex-direction: column;
  gap: 20px;
}

/* Header */
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-bottom: 4px;
}

.header-title-box {
  display: flex;
  align-items: center;
  gap: 10px;
}

.app-icon {
  width: 28px;
  height: 28px;
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

.header-badges {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-badge.ready {
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

.settings-gear-btn {
  background: transparent;
  border: none;
  color: #94a3b8;
  cursor: pointer;
  padding: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  transition: all 0.15s;
}

.settings-gear-btn:hover {
  color: #0f172a;
  background: #f1f5f9;
}

.btn-back-link {
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
.btn-back-link:hover {
  background: #0f172a;
  color: #fff;
  border-color: #0f172a;
}

/* Toast */
.status-toast {
  padding: 8px 14px;
  border-radius: 8px;
  font-size: 13px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.status-toast.success { background: #dcfce7; color: #15803d; }
.status-toast.warning { background: #fef9c3; color: #a16207; }
.status-toast.error { background: #fee2e2; color: #b91c1c; }
.status-toast.info { background: #e0f2fe; color: #0369a1; }
.toast-close { background: none; border: none; font-size: 16px; cursor: pointer; color: inherit; }

/* SECTION 1: GROUP SELECTOR */
.section-group {
  display: flex;
  flex-direction: column;
}

.label-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.section-label {
  font-size: 13px;
  color: #64748b;
  font-weight: 500;
}

.group-delete-link {
  background: none;
  border: none;
  color: #ef4444;
  font-size: 12px;
  cursor: pointer;
  padding: 0;
}
.group-delete-link:hover {
  text-decoration: underline;
}

.group-select-row {
  display: flex;
  gap: 10px;
  align-items: center;
}

.custom-select-wrap {
  flex: 1;
  position: relative;
  display: flex;
  align-items: center;
}

.group-select {
  width: 100%;
  appearance: none;
  background: #ffffff;
  border: 1px solid #cbd5e1;
  border-radius: 10px;
  padding: 10px 36px 10px 14px;
  font-size: 15px;
  font-weight: 500;
  color: #0f172a;
  outline: none;
  cursor: pointer;
  transition: border-color 0.15s;
}

.group-select:focus {
  border-color: #0f172a;
}

.empty-select-placeholder {
  width: 100%;
  border: 1px dashed #cbd5e1;
  border-radius: 10px;
  padding: 10px 14px;
  font-size: 14px;
  color: #94a3b8;
  cursor: pointer;
}

.select-chevron {
  position: absolute;
  right: 14px;
  pointer-events: none;
  color: #64748b;
}

.add-group-btn {
  width: 44px;
  height: 44px;
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

.add-group-btn:hover {
  background: #f8fafc;
  border-color: #94a3b8;
  color: #0f172a;
}

.settings-file-text {
  margin-top: 6px;
  font-size: 13px;
  color: #64748b;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.clickable-file {
  cursor: pointer;
}
.clickable-file:hover {
  text-decoration: underline;
  color: #0f172a;
}

.empty-file-hint {
  color: #94a3b8;
}

/* SECTION 2: RELAY NODES */
.section-nodes {
  display: flex;
  flex-direction: column;
}

.nodes-header-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 12px;
}

.nodes-title {
  margin: 0;
  font-size: 15px;
  font-weight: 700;
  color: #0f172a;
}

.nodes-desc {
  font-size: 12px;
  color: #64748b;
}

.nodes-container {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.empty-nodes-box {
  padding: 24px;
  border: 1px dashed #cbd5e1;
  border-radius: 12px;
  text-align: center;
  color: #64748b;
}

.empty-nodes-box p {
  margin: 0 0 4px 0;
  font-weight: 500;
  font-size: 14px;
  color: #334155;
}

.empty-sub {
  font-size: 12px;
  color: #94a3b8;
}

/* Node Card */
.node-card {
  position: relative;
  display: flex;
  align-items: center;
  padding: 14px 18px;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  background: #ffffff;
  cursor: pointer;
  transition: all 0.15s;
}

.node-card:hover {
  border-color: #cbd5e1;
  background: #fafafa;
}

.node-card.active {
  border: 1.5px solid #10b981;
  background: #ffffff;
}

.node-icon-box {
  margin-right: 14px;
  display: flex;
  align-items: center;
}

.check-circle {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: #10b981;
  color: #ffffff;
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-circle {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #94a3b8;
}

.node-content {
  flex: 1;
}

.node-name-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.node-name {
  font-size: 15px;
  font-weight: 600;
  color: #0f172a;
}

.node-url {
  font-size: 13px;
  color: #64748b;
  margin-top: 2px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.node-meta-box {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
  margin-left: 12px;
}

.badge-current {
  background: #dcfce7;
  color: #15803d;
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 6px;
}

.btn-switch-text {
  font-size: 13px;
  color: #64748b;
}

.node-card:hover .btn-switch-text {
  color: #0f172a;
}

.latency-ms {
  font-size: 13px;
  color: #64748b;
}

.latency-testing {
  font-size: 12px;
  color: #3b82f6;
}

.latency-empty {
  font-size: 13px;
  color: #94a3b8;
}

.node-edit-icon {
  position: absolute;
  top: 10px;
  right: 10px;
  background: transparent;
  border: none;
  color: #cbd5e1;
  padding: 4px;
  border-radius: 4px;
  cursor: pointer;
  display: none;
}

.node-card:hover .node-edit-icon {
  display: block;
}

.node-edit-icon:hover {
  color: #0f172a;
  background: #f1f5f9;
}

/* Action Buttons below Nodes */
.nodes-action-row {
  display: flex;
  gap: 12px;
  margin-top: 12px;
}

.pill-btn {
  flex: 1;
  border: 1px solid #cbd5e1;
  border-radius: 9999px;
  background: #ffffff;
  color: #334155;
  font-size: 13px;
  font-weight: 500;
  padding: 10px 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  cursor: pointer;
  transition: all 0.15s;
}

.pill-btn:hover:not(:disabled) {
  border-color: #94a3b8;
  background: #f8fafc;
  color: #0f172a;
}

.pill-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-icon {
  font-size: 16px;
}

/* SECTION 3: LAUNCH CONFIG PREVIEW */
.section-preview {
  border-top: 1px solid #f1f5f9;
  padding-top: 20px;
  display: flex;
  flex-direction: column;
}

.preview-header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.preview-title {
  margin: 0;
  font-size: 15px;
  font-weight: 700;
  color: #0f172a;
}

.badge-session {
  background: #f1f5f9;
  color: #475569;
  font-size: 11px;
  font-weight: 500;
  padding: 3px 8px;
  border-radius: 6px;
}

.preview-subtitle {
  font-size: 13px;
  color: #475569;
  margin-top: 4px;
}

.preview-subtitle strong {
  color: #0f172a;
}

.text-muted {
  color: #94a3b8;
}

.command-box {
  margin: 16px 0 14px 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 13px;
  color: #475569;
  user-select: all;
  word-break: break-all;
}

.btn-copy-command {
  width: 100%;
  background: #0f172a;
  color: #ffffff;
  border: none;
  border-radius: 9999px;
  padding: 13px 20px;
  font-size: 14px;
  font-weight: 500;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-copy-command:hover {
  background: #1e293b;
}

.btn-copy-command.copied {
  background: #16a34a;
}

.footer-caption {
  font-size: 12px;
  color: #94a3b8;
  text-align: center;
  margin: 14px 0 0 0;
  line-height: 1.5;
}

/* ==================== SETTINGS PAGE STYLES ==================== */
.settings-card {
  gap: 22px;
}

.settings-group {
  display: flex;
  flex-direction: column;
}

.settings-label {
  font-size: 13px;
  color: #64748b;
  margin-bottom: 8px;
}

.settings-label-strong {
  font-size: 14px;
  font-weight: 600;
  color: #0f172a;
  margin-bottom: 10px;
}

/* OS Segmented Control */
.os-segmented-control {
  display: flex;
  background: #f1f5f9;
  padding: 3px;
  border-radius: 9999px;
  gap: 2px;
}

.os-segment {
  flex: 1;
  padding: 8px 16px;
  border-radius: 9999px;
  border: none;
  background: transparent;
  color: #64748b;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.os-segment.active {
  background: #ffffff;
  color: #0f172a;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

/* Radio Options Column */
.radio-options-column {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 14px;
}

.custom-radio-row {
  display: flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
}

.custom-radio-row input[type="radio"] {
  accent-color: #0f172a;
  width: 16px;
  height: 16px;
  cursor: pointer;
}

.radio-text {
  font-size: 14px;
  color: #1e293b;
  font-weight: 500;
}

.btn-select-dir {
  margin-left: auto;
  border: 1px solid #cbd5e1;
  background: #ffffff;
  color: #334155;
  font-size: 12px;
  padding: 4px 10px;
  border-radius: 6px;
  cursor: pointer;
}
.btn-select-dir:hover {
  background: #f8fafc;
  color: #0f172a;
}

/* Dir Preview Box */
.dir-preview-box {
  background: #f8fafc;
  border: 1px solid #f1f5f9;
  border-radius: 10px;
  padding: 12px 14px;
}

.dir-preview-header {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #475569;
  font-size: 13px;
  font-weight: 500;
  margin-bottom: 4px;
}

.dir-path-value {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 13px;
  color: #0f172a;
  margin-bottom: 4px;
  cursor: pointer;
  word-break: break-all;
}
.dir-path-value:hover {
  text-decoration: underline;
}

.dir-path-note {
  font-size: 11px;
  color: #94a3b8;
}

.section-divider-clean {
  border-top: 1px solid #f1f5f9;
  margin-bottom: 16px;
}

.settings-section-title {
  margin: 0 0 12px 0;
  font-size: 15px;
  font-weight: 700;
  color: #0f172a;
}

/* File Preview List */
.files-preview-list {
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  overflow: hidden;
  background: #ffffff;
}

.file-row-item {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid #f1f5f9;
  cursor: pointer;
  transition: background 0.15s;
}
.file-row-item:last-child {
  border-bottom: none;
}
.file-row-item:hover {
  background: #f8fafc;
}

.file-icon-wrap {
  margin-right: 12px;
  color: #64748b;
  display: flex;
  align-items: center;
}

.file-name-text {
  flex: 1;
  font-size: 14px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  color: #1e293b;
}

.badge-file {
  font-size: 11px;
  font-weight: 500;
  padding: 3px 8px;
  border-radius: 6px;
}
.badge-file.base {
  background: #f1f5f9;
  color: #475569;
}
.badge-file.group {
  background: #e6f9ed;
  color: #16a34a;
}
.badge-file.other {
  background: #e0f2fe;
  color: #0369a1;
}

/* MODALS */
.modal-backdrop {
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

.modal-card {
  background: #ffffff;
  border-radius: 14px;
  width: 440px;
  max-width: 90vw;
  box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
  display: flex;
  flex-direction: column;
}

.modal-wide {
  width: 540px;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid #f1f5f9;
}

.modal-title {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: #0f172a;
}

.modal-close {
  background: none;
  border: none;
  font-size: 20px;
  color: #94a3b8;
  cursor: pointer;
}

.modal-body {
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-height: 70vh;
  overflow-y: auto;
}

.form-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.form-label {
  font-size: 13px;
  font-weight: 500;
  color: #334155;
}

.form-input {
  width: 100%;
  border: 1px solid #cbd5e1;
  border-radius: 8px;
  padding: 8px 12px;
  font-size: 14px;
  color: #0f172a;
  outline: none;
  box-sizing: border-box;
}

.form-input:focus {
  border-color: #0f172a;
}

.slug-row {
  display: flex;
  align-items: center;
  border: 1px solid #cbd5e1;
  border-radius: 8px;
  overflow: hidden;
}

.slug-tag {
  background: #f8fafc;
  padding: 8px 10px;
  color: #64748b;
  font-size: 13px;
  font-family: monospace;
}

.slug-input {
  border: none;
  padding: 8px;
  font-family: monospace;
  font-size: 14px;
}

.slug-input:focus {
  outline: none;
}

.form-hint {
  font-size: 11px;
  color: #64748b;
}

.form-grid {
  display: flex;
  gap: 12px;
}
.form-grid .form-item {
  flex: 1;
}

.password-wrap {
  position: relative;
  display: flex;
  align-items: center;
}

.toggle-eye {
  position: absolute;
  right: 10px;
  background: none;
  border: none;
  font-size: 12px;
  color: #64748b;
  cursor: pointer;
}

.form-divider {
  border-bottom: 1px solid #f1f5f9;
  line-height: 0.1em;
  margin: 10px 0;
  text-align: center;
}
.form-divider span {
  background: #fff;
  padding: 0 10px;
  color: #94a3b8;
  font-size: 12px;
}

.json-textarea {
  width: 100%;
  border: 1px solid #cbd5e1;
  border-radius: 8px;
  font-family: monospace;
  font-size: 12px;
  padding: 10px;
  box-sizing: border-box;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 10px;
  padding: 14px 20px;
  border-top: 1px solid #f1f5f9;
}

.footer-right {
  display: flex;
  gap: 10px;
}

.btn-cancel {
  border: 1px solid #cbd5e1;
  background: #ffffff;
  color: #475569;
  padding: 7px 14px;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
}

.btn-confirm {
  border: none;
  background: #0f172a;
  color: #ffffff;
  padding: 7px 16px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
}

.btn-danger-link {
  margin-right: auto;
  background: none;
  border: none;
  color: #ef4444;
  font-size: 13px;
  cursor: pointer;
  padding: 0;
}
</style>
