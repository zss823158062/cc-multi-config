<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { api } from './api';
import type {
  Group,
  RelayNode,
  Preferences,
  NodeTestResult,
  ConfigDirInfo,
  ClaudeBinaryInfo,
  TerminalInfo,
  SettingsDiffResult,
  BackupItem,
} from './types';

// State
const configDirInfo = ref<ConfigDirInfo | null>(null);
const claudeBinary = ref<ClaudeBinaryInfo | null>(null);
const terminals = ref<TerminalInfo[]>([]);
const groups = ref<Group[]>([]);
const nodes = ref<RelayNode[]>([]);
const preferences = ref<Preferences>({
  configDirMode: 'auto',
  customClaudeConfigDir: null,
  terminal: 'system',
  claudeExecutablePath: null,
  lastProjectDir: null,
});

const currentGroupId = ref<string>('');
const projectDir = ref<string>('');
const selectedTerminal = ref<string>('system');

// Testing State
const nodeTestingState = ref<Record<string, { testing: boolean; result?: NodeTestResult }>>({});
const isTestingAll = ref(false);

// Notification banner
const statusMessage = ref<{ type: 'info' | 'success' | 'warning' | 'error'; text: string } | null>(null);
const sessionNotice = ref<string | null>(null);

// Modals
const showGroupModal = ref(false);
const showNodeModal = ref(false);
const showDiffModal = ref(false);
const showSettingsModal = ref(false);
const showJsonModal = ref(false);
const showManageNodesModal = ref(false);

// Editing models
const editingGroup = ref<{ name: string; slug: string; initialNodeId: string }>({
  name: '',
  slug: '',
  initialNodeId: '',
});

const editingNode = ref<{
  isNew: boolean;
  node: RelayNode;
  applyToGroups: boolean;
}>({
  isNew: true,
  node: {
    id: '',
    name: '',
    baseUrl: '',
    authMode: 'authToken',
    authToken: '',
    models: {
      opus: '',
      sonnet: '',
      haiku: '',
      defaultModel: '',
    },
  },
  applyToGroups: true,
});

const showPassword = ref(false);
const currentDiff = ref<SettingsDiffResult | null>(null);
const currentJsonContent = ref<string>('');
const backups = ref<BackupItem[]>([]);
const isLaunching = ref(false);

// Computed
const currentGroup = computed(() => {
  return groups.value.find((g) => g.id === currentGroupId.value) || groups.value[0] || null;
});

const activeNodeForCurrentGroup = computed(() => {
  if (!currentGroup.value?.activeNodeId) return null;
  return nodes.value.find((n) => n.id === currentGroup.value?.activeNodeId) || null;
});

function setStatus(text: string, type: 'info' | 'success' | 'warning' | 'error' = 'info', timeoutMs = 4000) {
  statusMessage.value = { type, text };
  if (timeoutMs > 0) {
    setTimeout(() => {
      if (statusMessage.value?.text === text) {
        statusMessage.value = null;
      }
    }, timeoutMs);
  }
}

// Lifecycle
onMounted(async () => {
  await reloadAllData();
});

async function reloadAllData() {
  try {
    const [cfg, allNodes, dirInfo, binaryInfo, termList] = await Promise.all([
      api.getLauncherConfig(),
      api.getNodes(),
      api.getClaudeConfigDirInfo(),
      api.detectClaudeBinary(),
      api.getAvailableTerminals(),
    ]);

    groups.value = cfg.groups;
    nodes.value = allNodes;
    preferences.value = cfg.preferences;
    configDirInfo.value = dirInfo;
    claudeBinary.value = binaryInfo;
    terminals.value = termList;

    if (cfg.preferences.terminal) {
      selectedTerminal.value = cfg.preferences.terminal;
    }
    if (cfg.preferences.lastProjectDir) {
      projectDir.value = cfg.preferences.lastProjectDir;
    }

    if (groups.value.length > 0 && !currentGroupId.value) {
      currentGroupId.value = groups.value[0].id;
    }
  } catch (err: any) {
    setStatus(`加载配置失败: ${err}`, 'error', 6000);
  }
}

// Group Actions
function openCreateGroupModal() {
  editingGroup.value = {
    name: '',
    slug: '',
    initialNodeId: nodes.value[0]?.id || '',
  };
  showGroupModal.value = true;
}

function onGroupNameInput() {
  // Auto-generate slug from name
  const raw = editingGroup.value.name.toLowerCase().trim();
  editingGroup.value.slug = raw
    .replace(/[^a-z0-9-_]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
}

async function handleCreateGroup() {
  if (!editingGroup.value.name.trim()) {
    setStatus('请输入分组名称', 'warning');
    return;
  }
  try {
    const newGroup = await api.createGroup(
      editingGroup.value.name.trim(),
      editingGroup.value.slug.trim() || undefined,
      editingGroup.value.initialNodeId || undefined
    );
    groups.value.push(newGroup);
    currentGroupId.value = newGroup.id;
    showGroupModal.value = false;
    setStatus(`成功创建分组 ${newGroup.name} (${newGroup.settingsFile})`, 'success');
  } catch (err: any) {
    setStatus(`创建分组失败: ${err}`, 'error', 6000);
  }
}

async function handleDeleteCurrentGroup() {
  if (!currentGroup.value) return;
  if (groups.value.length <= 1) {
    setStatus('至少保留一个分组，无法删除最后一个分组', 'warning');
    return;
  }
  const confirmed = confirm(
    `确定删除分组 "${currentGroup.value.name}" 吗？\n是否同时删除其配置文件 ${currentGroup.value.settingsFile}？\n\n点击【确定】删除分组并删除文件；点击【取消】中止操作。`
  );
  if (!confirmed) return;

  try {
    const toDeleteId = currentGroup.value.id;
    await api.deleteGroup(toDeleteId, true);
    groups.value = groups.value.filter((g) => g.id !== toDeleteId);
    currentGroupId.value = groups.value[0].id;
    setStatus(`已删除分组 ${toDeleteId}`, 'info');
  } catch (err: any) {
    setStatus(`删除分组失败: ${err}`, 'error');
  }
}

// Node Actions
async function handleSwitchNode(nodeId: string) {
  if (!currentGroup.value) return;
  if (currentGroup.value.activeNodeId === nodeId) return;

  try {
    await api.switchGroupNode(currentGroup.value.id, nodeId);
    currentGroup.value.activeNodeId = nodeId;
    sessionNotice.value = '已切换中转节点并写入配置文件。注意：已在运行的终端会话需重启后方可生效。';
    setStatus(`已将分组 ${currentGroup.value.name} 切换至节点`, 'success');
  } catch (err: any) {
    setStatus(`切换节点失败: ${err}`, 'error', 6000);
  }
}

function openAddNodeModal() {
  const newId = `node-${Date.now().toString(36)}`;
  editingNode.value = {
    isNew: true,
    node: {
      id: newId,
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
    applyToGroups: true,
  };
  showNodeModal.value = true;
}

function openEditNodeModal(node: RelayNode) {
  editingNode.value = {
    isNew: false,
    node: JSON.parse(JSON.stringify(node)),
    applyToGroups: true,
  };
  showNodeModal.value = true;
}

async function handleSaveNode() {
  const n = editingNode.value.node;
  if (!n.name.trim() || !n.baseUrl.trim()) {
    setStatus('请填写节点名称和 Base URL', 'warning');
    return;
  }
  try {
    const saved = await api.saveNode(n, editingNode.value.applyToGroups);
    const idx = nodes.value.findIndex((item) => item.id === saved.id);
    if (idx >= 0) {
      nodes.value[idx] = saved;
    } else {
      nodes.value.push(saved);
    }
    showNodeModal.value = false;
    setStatus(`节点 "${saved.name}" 保存成功`, 'success');
  } catch (err: any) {
    setStatus(`保存节点失败: ${err}`, 'error');
  }
}

async function handleDeleteNode(nodeId: string) {
  if (confirm('确定要删除该中转节点吗？')) {
    try {
      await api.deleteNode(nodeId);
      nodes.value = nodes.value.filter((n) => n.id !== nodeId);
      // update groups in memory
      for (const g of groups.value) {
        if (g.activeNodeId === nodeId) {
          g.activeNodeId = null;
        }
      }
      setStatus('中转节点已删除', 'info');
    } catch (err: any) {
      setStatus(`删除节点失败: ${err}`, 'error');
    }
  }
}

// Node Testing
async function handleTestNode(node: RelayNode, testModel = false) {
  nodeTestingState.value[node.id] = { testing: true };
  try {
    const res = await api.testNode(node, testModel);
    nodeTestingState.value[node.id] = { testing: false, result: res };
    if (res.connectivityOk) {
      setStatus(`节点 ${node.name} 测试完成: ${res.message}`, 'success');
    } else {
      setStatus(`节点 ${node.name} 测试异常: ${res.message}`, 'warning');
    }
  } catch (err: any) {
    nodeTestingState.value[node.id] = {
      testing: false,
      result: {
        nodeId: node.id,
        connectivityOk: false,
        message: `测试失败: ${err}`,
      },
    };
  }
}

async function handleTestAllNodes() {
  if (isTestingAll.value) return;
  isTestingAll.value = true;
  setStatus('正在批量测试所有中转节点...', 'info');

  for (const node of nodes.value) {
    nodeTestingState.value[node.id] = { testing: true };
  }

  try {
    const results = await api.testAllNodes(false);
    for (const res of results) {
      nodeTestingState.value[res.nodeId] = { testing: false, result: res };
    }
    setStatus('所有节点测速完成', 'success');
  } catch (err: any) {
    setStatus(`批量测试失败: ${err}`, 'error');
  } finally {
    isTestingAll.value = false;
  }
}

// Directory & Launching
async function handleSelectProjectFolder() {
  try {
    const selected = await api.selectFolder(projectDir.value || undefined);
    if (selected) {
      projectDir.value = selected;
      preferences.value.lastProjectDir = selected;
      await api.savePreferences(preferences.value);
    }
  } catch (err: any) {
    setStatus(`选择文件夹失败: ${err}`, 'error');
  }
}

async function handleOpenProjectInExplorer() {
  if (!projectDir.value) {
    setStatus('请先指定项目工作目录', 'warning');
    return;
  }
  try {
    await api.openInExplorer(projectDir.value);
  } catch (err: any) {
    setStatus(`打开目录失败: ${err}`, 'error');
  }
}

async function handleOpenConfigDir() {
  if (configDirInfo.value?.resolvedPath) {
    try {
      await api.openInExplorer(configDirInfo.value.resolvedPath);
    } catch (err: any) {
      setStatus(`打开配置目录失败: ${err}`, 'error');
    }
  }
}

async function handleOpenSettingsFile() {
  if (!currentGroup.value || !configDirInfo.value) return;
  const filePath = `${configDirInfo.value.resolvedPath}/${currentGroup.value.settingsFile}`;
  try {
    await api.openFileInEditor(filePath);
  } catch (err: any) {
    setStatus(`打开设置文件失败: ${err}`, 'error');
  }
}

async function handlePreviewDiff(nodeId?: string) {
  if (!currentGroup.value) return;
  const targetNodeId = nodeId || currentGroup.value.activeNodeId || nodes.value[0]?.id;
  if (!targetNodeId) {
    setStatus('请先选择中转节点', 'warning');
    return;
  }
  try {
    const diff = await api.previewSettingsDiff(currentGroup.value.id, targetNodeId);
    currentDiff.value = diff;
    showDiffModal.value = true;
  } catch (err: any) {
    setStatus(`获取差异预览失败: ${err}`, 'error');
  }
}

async function handleViewJson() {
  if (!currentGroup.value) return;
  try {
    const content = await api.getGroupSettingsContent(currentGroup.value.id);
    currentJsonContent.value = content;
    showJsonModal.value = true;
  } catch (err: any) {
    setStatus(`读取 JSON 失败: ${err}`, 'error');
  }
}

async function handleSaveJson() {
  if (!currentGroup.value) return;
  try {
    await api.saveGroupSettingsContent(currentGroup.value.id, currentJsonContent.value);
    showJsonModal.value = false;
    setStatus('分组配置文件保存成功', 'success');
  } catch (err: any) {
    setStatus(`保存 JSON 失败: ${err}`, 'error');
  }
}

async function handleLaunchClaude() {
  if (!currentGroup.value) {
    setStatus('请先选择模型分组', 'warning');
    return;
  }
  if (!projectDir.value.trim()) {
    setStatus('请先选择或输入项目工作目录', 'warning');
    return;
  }
  if (!currentGroup.value.activeNodeId) {
    setStatus('当前分组尚未关联中转节点，请在列表中选择一个节点', 'warning');
    return;
  }

  isLaunching.value = true;
  try {
    const res = await api.launchClaude(
      currentGroup.value.id,
      projectDir.value.trim(),
      selectedTerminal.value
    );
    setStatus(`🚀 ${res.message}`, 'success', 5000);
  } catch (err: any) {
    setStatus(`启动 Claude Code 失败: ${err}`, 'error', 7000);
  } finally {
    isLaunching.value = false;
  }
}

// Settings & Backups
async function openSettings() {
  try {
    const [backupsList] = await Promise.all([
      api.listBackups(),
      api.detectClaudeBinary(preferences.value.claudeExecutablePath || undefined),
    ]);
    backups.value = backupsList;
    showSettingsModal.value = true;
  } catch (err: any) {
    setStatus(`打开设置失败: ${err}`, 'error');
  }
}

async function handleSavePreferences() {
  try {
    preferences.value.terminal = selectedTerminal.value;
    await api.savePreferences(preferences.value);
    await reloadAllData();
    showSettingsModal.value = false;
    setStatus('启动器偏好设置已更新', 'success');
  } catch (err: any) {
    setStatus(`保存偏好设置失败: ${err}`, 'error');
  }
}

async function handleRestoreBackup(backupItem: BackupItem) {
  if (confirm(`确定要从备份 ${backupItem.filename} 还原分组 "${backupItem.groupId}" 的配置吗？当前内容将自动创建安全备份。`)) {
    try {
      await api.restoreBackup(backupItem.filename, backupItem.groupId);
      setStatus('备份已成功还原', 'success');
      showSettingsModal.value = false;
    } catch (err: any) {
      setStatus(`还原备份失败: ${err}`, 'error');
    }
  }
}

async function handleCreateEmptyBaseSettings() {
  try {
    const p = await api.createEmptyBaseSettings();
    setStatus(`成功创建基础 settings.json: ${p}`, 'success');
    await reloadAllData();
  } catch (err: any) {
    setStatus(`创建失败: ${err}`, 'error');
  }
}
</script>

<template>
  <div class="app-layout">
    <!-- TOP HEADER -->
    <header class="app-header">
      <div class="header-left">
        <div class="app-brand">
          <div class="brand-logo">
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M4 17l6-6-6-6M12 19h8" />
            </svg>
          </div>
          <div class="brand-info">
            <h1 class="brand-title">Claude多实例配置启动器</h1>
            <span class="brand-sub">Multi-Instance Claude Code Launcher</span>
          </div>
        </div>
      </div>

      <div class="header-right">
        <!-- Claude Executable Status -->
        <div
          class="status-chip"
          :class="claudeBinary?.isDetected ? 'chip-success' : 'chip-warning'"
          :title="claudeBinary?.path || '未检测到 Claude Code'"
        >
          <span class="status-dot"></span>
          <span>Claude: {{ claudeBinary?.isDetected ? (claudeBinary.version || '已就绪') : '未检测到' }}</span>
        </div>

        <!-- Config Dir Chip -->
        <button class="status-chip chip-neutral" @click="handleOpenConfigDir" title="点击打开配置目录">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
          </svg>
          <span class="truncate-text">{{ configDirInfo?.resolvedPath || '~/.claude' }}</span>
        </button>

        <!-- Settings Gear -->
        <button class="icon-btn" @click="openSettings" title="启动器全局设置">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="3"></circle>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
          </svg>
        </button>
      </div>
    </header>

    <!-- ALERT BANNER -->
    <div v-if="statusMessage" class="banner-alert" :class="`alert-${statusMessage.type}`">
      <span>{{ statusMessage.text }}</span>
      <button class="banner-close" @click="statusMessage = null">×</button>
    </div>

    <!-- MAIN DASHBOARD CONTENT -->
    <main class="app-body">
      <!-- 1. GROUP SELECTOR SECTION -->
      <section class="card group-card">
        <div class="card-header">
          <div class="card-title-group">
            <span class="card-step-badge">1</span>
            <div>
              <h2 class="card-title">模型分组 / 配置文件</h2>
              <p class="card-desc">每个模型分组拥有一份独立的配置文件，基础 settings.json 绝不被污染</p>
            </div>
          </div>
          <div class="header-actions">
            <button class="btn btn-primary-soft btn-sm" @click="openCreateGroupModal">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line>
              </svg>
              新建分组
            </button>
            <button class="btn btn-ghost btn-sm text-danger" @click="handleDeleteCurrentGroup" title="删除当前分组">
              删除分组
            </button>
          </div>
        </div>

        <!-- Groups Tab Bar -->
        <div class="group-tabs">
          <button
            v-for="grp in groups"
            :key="grp.id"
            class="group-tab"
            :class="{ active: grp.id === currentGroupId }"
            @click="currentGroupId = grp.id"
          >
            <span class="group-tab-name">{{ grp.name }}</span>
            <span class="group-tab-badge">{{ grp.settingsFile }}</span>
          </button>
        </div>

        <!-- Group detail bar -->
        <div class="group-detail-bar" v-if="currentGroup">
          <div class="file-path-tag">
            <span class="label">加载文件：</span>
            <code class="code-badge" @click="handleOpenSettingsFile" title="点击在系统默认程序中打开">
              {{ currentGroup.settingsFile }}
            </code>
            <button class="text-link" @click="handleOpenConfigDir" title="在系统资源管理器中定位">
              打开所在目录
            </button>
          </div>

          <div class="preview-actions">
            <button class="btn btn-outline btn-xs" @click="handleViewJson">
              查看/编辑 JSON
            </button>
            <button class="btn btn-outline btn-xs" @click="handlePreviewDiff()">
              配置差异预览
            </button>
          </div>
        </div>
      </section>

      <!-- 2. RELAY NODES SECTION -->
      <section class="card nodes-card">
        <div class="card-header">
          <div class="card-title-group">
            <span class="card-step-badge">2</span>
            <div>
              <h2 class="card-title">中转节点 (Relay Nodes)</h2>
              <p class="card-desc">为当前分组 <strong>「{{ currentGroup?.name }}」</strong> 选择使用的 API 中转连接节点</p>
            </div>
          </div>
          <div class="header-actions">
            <button class="btn btn-outline btn-sm" :disabled="isTestingAll" @click="handleTestAllNodes">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline>
              </svg>
              {{ isTestingAll ? '测试中...' : '测速全部' }}
            </button>
            <button class="btn btn-outline btn-sm" @click="showManageNodesModal = true">
              管理节点
            </button>
            <button class="btn btn-primary-soft btn-sm" @click="openAddNodeModal">
              + 添加节点
            </button>
          </div>
        </div>

        <!-- Node Cards Grid -->
        <div class="node-list">
          <div
            v-for="node in nodes"
            :key="node.id"
            class="node-card"
            :class="{ active: currentGroup?.activeNodeId === node.id }"
            @click="handleSwitchNode(node.id)"
          >
            <div class="node-radio">
              <input
                type="radio"
                name="activeNode"
                :checked="currentGroup?.activeNodeId === node.id"
                @change="handleSwitchNode(node.id)"
              />
            </div>

            <div class="node-main">
              <div class="node-header">
                <span class="node-name">{{ node.name }}</span>
                <span v-if="currentGroup?.activeNodeId === node.id" class="badge-active">当前启用</span>
              </div>
              <div class="node-url" :title="node.baseUrl">
                {{ node.baseUrl }}
              </div>
              <div class="node-models-tags">
                <span v-if="node.models.opus" class="tag">Opus: {{ node.models.opus }}</span>
                <span v-if="node.models.sonnet" class="tag">Sonnet: {{ node.models.sonnet }}</span>
                <span v-if="node.models.haiku" class="tag">Haiku: {{ node.models.haiku }}</span>
              </div>
            </div>

            <div class="node-right" @click.stop>
              <!-- Latency & Status -->
              <div class="node-status">
                <div v-if="nodeTestingState[node.id]?.testing" class="status-pill testing">
                  <span class="spinner-sm"></span> 测速中...
                </div>
                <div
                  v-else-if="nodeTestingState[node.id]?.result"
                  class="status-pill"
                  :class="nodeTestingState[node.id].result?.connectivityOk ? 'ok' : 'fail'"
                  :title="nodeTestingState[node.id].result?.message"
                >
                  <span v-if="nodeTestingState[node.id].result?.latencyMs !== undefined">
                    {{ nodeTestingState[node.id].result?.latencyMs }}ms
                  </span>
                  <span v-else>
                    {{ nodeTestingState[node.id].result?.connectivityOk ? '可用' : '失败' }}
                  </span>
                </div>
                <div v-else class="status-pill untested">
                  未测试
                </div>
              </div>

              <!-- Action buttons -->
              <div class="node-actions">
                <button class="btn btn-ghost btn-xs" @click="handleTestNode(node, false)" title="测试连通性">
                  测速
                </button>
                <button class="btn btn-ghost btn-xs" @click="openEditNodeModal(node)" title="编辑节点参数">
                  编辑
                </button>
              </div>
            </div>
          </div>
        </div>

        <div v-if="sessionNotice" class="session-notice-box">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line>
          </svg>
          <span>{{ sessionNotice }}</span>
        </div>
      </section>

      <!-- 3. PROJECT DIRECTORY SECTION -->
      <section class="card project-card">
        <div class="card-header">
          <div class="card-title-group">
            <span class="card-step-badge">3</span>
            <div>
              <h2 class="card-title">项目工作目录</h2>
              <p class="card-desc">指定 Claude Code 启动时的根目录，将自动以该目录启动终端</p>
            </div>
          </div>
        </div>

        <div class="project-input-row">
          <div class="input-wrap">
            <svg class="input-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
            </svg>
            <input
              type="text"
              class="form-control"
              v-model="projectDir"
              placeholder="例如：E:\projects\my-app 或 /Users/name/projects"
            />
          </div>
          <button class="btn btn-outline" @click="handleSelectProjectFolder">
            选择目录
          </button>
          <button class="btn btn-ghost" @click="handleOpenProjectInExplorer" title="在系统资源管理器中打开">
            浏览
          </button>
        </div>
      </section>

      <!-- 4. LAUNCH & CONTROLS FOOTER -->
      <section class="launch-card">
        <div class="launch-settings">
          <div class="terminal-selector">
            <label class="field-label">终端类型：</label>
            <select v-model="selectedTerminal" class="select-control">
              <option value="system">系统自适应推荐 (Windows Terminal)</option>
              <option v-for="t in terminals" :key="t.id" :value="t.id" :disabled="!t.isAvailable">
                {{ t.name }} {{ !t.isAvailable ? '(未安装)' : '' }}
              </option>
            </select>
          </div>

          <div class="active-summary">
            <span class="summary-dot"></span>
            <span>
              已就绪：分组 <strong>{{ currentGroup?.name }}</strong> ·
              节点 <strong>{{ activeNodeForCurrentGroup?.name || '未选节点' }}</strong>
            </span>
          </div>
        </div>

        <div class="launch-action">
          <button
            class="btn btn-launch"
            :disabled="isLaunching || !activeNodeForCurrentGroup || !projectDir"
            @click="handleLaunchClaude"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polygon points="5 3 19 12 5 21 5 3"></polygon>
            </svg>
            <span>{{ isLaunching ? '正在启动...' : `在终端启动 Claude Code (${currentGroup?.name || ''})` }}</span>
          </button>
        </div>
      </section>
    </main>

    <!-- STATUS BAR -->
    <footer class="app-statusbar">
      <div class="status-left">
        <span>配置状态：</span>
        <span class="status-value">{{ currentGroup?.settingsFile }} 已同步</span>
        <span class="status-separator">|</span>
        <span>当前节点：</span>
        <span class="status-value">{{ activeNodeForCurrentGroup?.name || '无' }}</span>
      </div>
      <div class="status-right">
        <span>Claude Multi-Instance Launcher v1.0 (MVP)</span>
      </div>
    </footer>

    <!-- MODAL: CREATE GROUP -->
    <div v-if="showGroupModal" class="modal-backdrop" @click.self="showGroupModal = false">
      <div class="modal-dialog">
        <div class="modal-header">
          <h3 class="modal-title">新建模型分组</h3>
          <button class="modal-close" @click="showGroupModal = false">×</button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label class="form-label">展示名称：</label>
            <input
              type="text"
              class="form-control"
              v-model="editingGroup.name"
              @input="onGroupNameInput"
              placeholder="例如：Gemini-3-Pro 或 Claude-Opus"
              autofocus
            />
          </div>

          <div class="form-group">
            <label class="form-label">文件名标识 (Slug)：</label>
            <div class="slug-preview">
              <span class="slug-prefix">my-</span>
              <input type="text" class="form-control slug-input" v-model="editingGroup.slug" placeholder="gemini-3-pro" />
              <span class="slug-suffix">.json</span>
            </div>
            <p class="field-hint">将自动在 Claude 配置目录创建独立设置文件，不修改原有 settings.json</p>
          </div>

          <div class="form-group">
            <label class="form-label">初始中转节点：</label>
            <select class="form-control" v-model="editingGroup.initialNodeId">
              <option value="">暂不指定 (保持空连接)</option>
              <option v-for="node in nodes" :key="node.id" :value="node.id">
                {{ node.name }} ({{ node.baseUrl }})
              </option>
            </select>
          </div>

          <div class="info-callout">
            <div class="info-title">💡 配置隔离说明</div>
            <p class="info-desc">
              系统将复制基础配置的 permissions、hooks 等自定义设置，并清除既有的模型与连接字段，确保新分组完全独立。
            </p>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-outline" @click="showGroupModal = false">取消</button>
          <button class="btn btn-primary" @click="handleCreateGroup">确认创建</button>
        </div>
      </div>
    </div>

    <!-- MODAL: ADD/EDIT NODE -->
    <div v-if="showNodeModal" class="modal-backdrop" @click.self="showNodeModal = false">
      <div class="modal-dialog modal-lg">
        <div class="modal-header">
          <h3 class="modal-title">{{ editingNode.isNew ? '添加中转节点' : '编辑中转节点' }}</h3>
          <button class="modal-close" @click="showNodeModal = false">×</button>
        </div>
        <div class="modal-body">
          <div class="form-row">
            <div class="form-group flex-1">
              <label class="form-label">节点名称：</label>
              <input
                type="text"
                class="form-control"
                v-model="editingNode.node.name"
                placeholder="例如：中转站 A (备用线路)"
              />
            </div>
            <div class="form-group flex-1">
              <label class="form-label">认证模式：</label>
              <select class="form-control" v-model="editingNode.node.authMode">
                <option value="authToken">ANTHROPIC_AUTH_TOKEN (中转站通用推荐)</option>
                <option value="apiKey">ANTHROPIC_API_KEY (标准 API Key)</option>
              </select>
            </div>
          </div>

          <div class="form-group">
            <label class="form-label">API Base URL：</label>
            <input
              type="text"
              class="form-control"
              v-model="editingNode.node.baseUrl"
              placeholder="例如：https://api.example.com 或 https://api.anthropic.com"
            />
          </div>

          <div class="form-group">
            <label class="form-label">认证密钥 / Token：</label>
            <div class="input-wrap">
              <input
                :type="showPassword ? 'text' : 'password'"
                class="form-control"
                v-model="editingNode.node.authToken"
                placeholder="sk-..."
              />
              <button class="eye-btn" type="button" @click="showPassword = !showPassword">
                {{ showPassword ? '隐藏' : '显示' }}
              </button>
            </div>
            <p class="field-hint text-warning">
              ⚠️ 说明：由于 Claude Code 需要从 settings JSON 读取凭据，切换至此节点时 Token 将落盘至当前分组的 settings 文件中。
            </p>
          </div>

          <div class="section-divider">
            <span>模型映射 (按分组覆盖)</span>
          </div>

          <div class="form-row">
            <div class="form-group flex-1">
              <label class="form-label">默认模型 (ANTHROPIC_MODEL)：</label>
              <input
                type="text"
                class="form-control"
                v-model="editingNode.node.models.defaultModel"
                placeholder="claude-3-7-sonnet-20250219"
              />
            </div>
            <div class="form-group flex-1">
              <label class="form-label">Sonnet 映射：</label>
              <input
                type="text"
                class="form-control"
                v-model="editingNode.node.models.sonnet"
                placeholder="claude-3-7-sonnet-20250219"
              />
            </div>
          </div>

          <div class="form-row">
            <div class="form-group flex-1">
              <label class="form-label">Opus 映射：</label>
              <input
                type="text"
                class="form-control"
                v-model="editingNode.node.models.opus"
                placeholder="claude-3-opus-20240229"
              />
            </div>
            <div class="form-group flex-1">
              <label class="form-label">Haiku 映射：</label>
              <input
                type="text"
                class="form-control"
                v-model="editingNode.node.models.haiku"
                placeholder="claude-3-5-haiku-20241022"
              />
            </div>
          </div>

          <div class="form-checkbox">
            <label class="checkbox-label">
              <input type="checkbox" v-model="editingNode.applyToGroups" />
              <span>同步将此节点的变更应用到所有正使用该节点的模型分组</span>
            </label>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn btn-outline" @click="handleTestNode(editingNode.node, false)">
            测试连通性
          </button>
          <button class="btn btn-outline" @click="showNodeModal = false">取消</button>
          <button class="btn btn-primary" @click="handleSaveNode">保存节点</button>
        </div>
      </div>
    </div>

    <!-- MODAL: MANAGE NODES -->
    <div v-if="showManageNodesModal" class="modal-backdrop" @click.self="showManageNodesModal = false">
      <div class="modal-dialog modal-lg">
        <div class="modal-header">
          <h3 class="modal-title">管理中转节点</h3>
          <button class="modal-close" @click="showManageNodesModal = false">×</button>
        </div>
        <div class="modal-body">
          <table class="data-table">
            <thead>
              <tr>
                <th>节点名称</th>
                <th>Base URL</th>
                <th>认证模式</th>
                <th>测速状态</th>
                <th style="width: 140px;">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="node in nodes" :key="node.id">
                <td class="font-medium">{{ node.name }}</td>
                <td class="text-sub">{{ node.baseUrl }}</td>
                <td><span class="badge">{{ node.authMode }}</span></td>
                <td>
                  <span v-if="nodeTestingState[node.id]?.result?.latencyMs">
                    {{ nodeTestingState[node.id].result?.latencyMs }}ms
                  </span>
                  <span v-else class="text-sub">未测试</span>
                </td>
                <td>
                  <div class="table-actions">
                    <button class="btn btn-ghost btn-xs" @click="openEditNodeModal(node)">编辑</button>
                    <button class="btn btn-ghost btn-xs text-danger" @click="handleDeleteNode(node.id)">删除</button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div class="modal-footer">
          <button class="btn btn-primary-soft" @click="openAddNodeModal">+ 新增节点</button>
          <button class="btn btn-outline" @click="showManageNodesModal = false">关闭</button>
        </div>
      </div>
    </div>

    <!-- MODAL: DIFF PREVIEW -->
    <div v-if="showDiffModal" class="modal-backdrop" @click.self="showDiffModal = false">
      <div class="modal-dialog modal-lg">
        <div class="modal-header">
          <h3 class="modal-title">配置差异对比 (Diff Preview)</h3>
          <button class="modal-close" @click="showDiffModal = false">×</button>
        </div>
        <div class="modal-body">
          <div class="diff-summary-box">
            <h4>托管字段变更摘要：</h4>
            <ul>
              <li v-for="(item, i) in currentDiff?.diffSummary" :key="i">{{ item }}</li>
            </ul>
          </div>

          <div class="diff-file-tag">
            <span>目标文件：{{ currentDiff?.targetFilePath }}</span>
          </div>

          <div class="json-code-box">
            <pre><code>{{ currentDiff?.previewContent }}</code></pre>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-outline" @click="showDiffModal = false">关闭</button>
        </div>
      </div>
    </div>

    <!-- MODAL: VIEW / EDIT JSON -->
    <div v-if="showJsonModal" class="modal-backdrop" @click.self="showJsonModal = false">
      <div class="modal-dialog modal-lg">
        <div class="modal-header">
          <h3 class="modal-title">查看 / 编辑分组配置 ({{ currentGroup?.settingsFile }})</h3>
          <button class="modal-close" @click="showJsonModal = false">×</button>
        </div>
        <div class="modal-body">
          <p class="field-hint">
            提示：可在此查看或微调非托管字段（如 permissions, hooks, mcpServers）。修改前系统会自动创建备份。
          </p>
          <textarea class="code-textarea" v-model="currentJsonContent" rows="18"></textarea>
        </div>
        <div class="modal-footer">
          <button class="btn btn-outline" @click="handleOpenSettingsFile">在本地编辑器中打开</button>
          <button class="btn btn-outline" @click="showJsonModal = false">取消</button>
          <button class="btn btn-primary" @click="handleSaveJson">保存更改</button>
        </div>
      </div>
    </div>

    <!-- MODAL: PREFERENCES & SETTINGS -->
    <div v-if="showSettingsModal" class="modal-backdrop" @click.self="showSettingsModal = false">
      <div class="modal-dialog modal-lg">
        <div class="modal-header">
          <h3 class="modal-title">启动器偏好设置 & 数据管理</h3>
          <button class="modal-close" @click="showSettingsModal = false">×</button>
        </div>
        <div class="modal-body">
          <!-- Section 1: Claude Config Dir -->
          <div class="settings-section">
            <h4 class="section-title">Claude 配置目录优先级</h4>
            <div class="form-radio-group">
              <label class="radio-label">
                <input type="radio" value="auto" v-model="preferences.configDirMode" />
                <span>自动检测 (优先 CLAUDE_CONFIG_DIR，其次默认 ~/.claude)</span>
              </label>
              <label class="radio-label">
                <input type="radio" value="custom" v-model="preferences.configDirMode" />
                <span>使用自定义目录</span>
              </label>
            </div>

            <div v-if="preferences.configDirMode === 'custom'" class="custom-dir-row">
              <input
                type="text"
                class="form-control"
                v-model="preferences.customClaudeConfigDir"
                placeholder="例如：D:\ClaudeConfig"
              />
            </div>

            <div class="dir-status-box">
              <span>当前解析路径：<code>{{ configDirInfo?.resolvedPath }}</code></span>
              <span v-if="!configDirInfo?.hasBaseSettings" class="text-warning">
                未检测到基础 settings.json
                <button class="btn btn-outline btn-xs" @click="handleCreateEmptyBaseSettings">创建空白基础配置</button>
              </span>
            </div>
          </div>

          <!-- Section 2: Claude Executable -->
          <div class="settings-section">
            <h4 class="section-title">Claude Code 可执行文件</h4>
            <div class="form-group">
              <label class="form-label">指定可执行文件路径 (留空则自动检测)：</label>
              <input
                type="text"
                class="form-control"
                v-model="preferences.claudeExecutablePath"
                placeholder="例如：C:\Users\...\.local\bin\claude.exe"
              />
              <p class="field-hint">
                当前检测到：{{ claudeBinary?.path || '未找到' }} (来源: {{ claudeBinary?.source }})
              </p>
            </div>
          </div>

          <!-- Section 3: Automatic Backups -->
          <div class="settings-section">
            <h4 class="section-title">配置历史备份 (Backups)</h4>
            <p class="field-hint">修改分组配置时自动保留最近 10 次备份，可一键回滚还原</p>
            <div class="backups-list" v-if="backups.length > 0">
              <div v-for="b in backups.slice(0, 8)" :key="b.filename" class="backup-row">
                <div class="backup-info">
                  <span class="backup-name">{{ b.filename }}</span>
                  <span class="backup-date">{{ b.timestamp }}</span>
                </div>
                <button class="btn btn-outline btn-xs" @click="handleRestoreBackup(b)">还原此备份</button>
              </div>
            </div>
            <div v-else class="text-sub empty-state">
              暂无备份文件
            </div>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn btn-outline" @click="showSettingsModal = false">取消</button>
          <button class="btn btn-primary" @click="handleSavePreferences">保存设置</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Dark Modern Theme - Obsidian Slate */
.app-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  background-color: #0e1117;
  color: #e6edf3;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  overflow: hidden;
  user-select: none;
}

/* HEADER */
.app-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 20px;
  background-color: #161b22;
  border-bottom: 1px solid #30363d;
}

.brand-logo {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border-radius: 8px;
  background: linear-gradient(135deg, #d97706, #b45309);
  color: #ffffff;
  margin-right: 12px;
}

.app-brand {
  display: flex;
  align-items: center;
}

.brand-title {
  font-size: 15px;
  font-weight: 600;
  margin: 0;
  color: #f0f6fc;
}

.brand-sub {
  font-size: 11px;
  color: #8b949e;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.status-chip {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  padding: 4px 10px;
  border-radius: 20px;
  border: 1px solid #30363d;
  background: #21262d;
  color: #c9d1d9;
  cursor: pointer;
}

.chip-success {
  background: rgba(35, 134, 54, 0.15);
  border-color: rgba(35, 134, 54, 0.4);
  color: #3fb950;
}

.chip-warning {
  background: rgba(210, 153, 34, 0.15);
  border-color: rgba(210, 153, 34, 0.4);
  color: #d29922;
}

.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background-color: currentColor;
}

.truncate-text {
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.icon-btn {
  background: none;
  border: 1px solid #30363d;
  color: #c9d1d9;
  padding: 6px;
  border-radius: 8px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.icon-btn:hover {
  background: #21262d;
  border-color: #8b949e;
}

/* BANNER */
.banner-alert {
  padding: 8px 18px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
}

.alert-success { background: #238636; color: #fff; }
.alert-warning { background: #9e6a03; color: #fff; }
.alert-error { background: #da3633; color: #fff; }
.alert-info { background: #1f6feb; color: #fff; }

.banner-close {
  background: none;
  border: none;
  color: #fff;
  font-size: 18px;
  cursor: pointer;
}

/* APP BODY */
.app-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.card {
  background-color: #161b22;
  border: 1px solid #30363d;
  border-radius: 10px;
  padding: 14px 18px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 12px;
}

.card-title-group {
  display: flex;
  gap: 10px;
  align-items: flex-start;
}

.card-step-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: #238636;
  color: #ffffff;
  font-size: 12px;
  font-weight: 700;
  margin-top: 1px;
}

.card-title {
  font-size: 15px;
  font-weight: 600;
  margin: 0;
  color: #f0f6fc;
}

.card-desc {
  font-size: 12px;
  color: #8b949e;
  margin: 2px 0 0 0;
}

.header-actions {
  display: flex;
  gap: 8px;
}

/* GROUPS TAB BAR */
.group-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;
}

.group-tab {
  display: flex;
  align-items: center;
  gap: 8px;
  background: #21262d;
  border: 1px solid #30363d;
  color: #c9d1d9;
  padding: 7px 14px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.2s;
}

.group-tab:hover {
  border-color: #8b949e;
}

.group-tab.active {
  background: rgba(56, 139, 253, 0.15);
  border-color: #58a6ff;
  color: #58a6ff;
  font-weight: 600;
}

.group-tab-badge {
  font-size: 11px;
  padding: 2px 6px;
  background: rgba(0, 0, 0, 0.3);
  border-radius: 4px;
  color: #8b949e;
}

.group-detail-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: #0d1117;
  padding: 8px 12px;
  border-radius: 6px;
  border: 1px dashed #30363d;
  font-size: 12px;
}

.file-path-tag {
  display: flex;
  align-items: center;
  gap: 8px;
}

.code-badge {
  background: #1f242c;
  padding: 2px 8px;
  border-radius: 4px;
  color: #79c0ff;
  font-family: monospace;
  cursor: pointer;
}

.code-badge:hover {
  text-decoration: underline;
}

.text-link {
  background: none;
  border: none;
  color: #58a6ff;
  cursor: pointer;
  padding: 0;
  font-size: 12px;
}

.text-link:hover {
  text-decoration: underline;
}

/* NODE CARDS */
.node-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.node-card {
  display: flex;
  align-items: center;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 10px 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.node-card:hover {
  border-color: #58a6ff;
}

.node-card.active {
  border-color: #238636;
  background: rgba(35, 134, 54, 0.08);
}

.node-radio {
  margin-right: 12px;
}

.node-main {
  flex: 1;
}

.node-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.node-name {
  font-weight: 600;
  font-size: 14px;
  color: #f0f6fc;
}

.badge-active {
  font-size: 11px;
  background: #238636;
  color: #fff;
  padding: 1px 6px;
  border-radius: 4px;
}

.node-url {
  font-size: 12px;
  color: #8b949e;
  font-family: monospace;
  margin-top: 2px;
}

.node-models-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 5px;
}

.tag {
  font-size: 11px;
  background: #21262d;
  color: #8b949e;
  padding: 1px 6px;
  border-radius: 4px;
}

.node-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.status-pill {
  font-size: 11px;
  padding: 3px 8px;
  border-radius: 12px;
  border: 1px solid transparent;
  display: flex;
  align-items: center;
  gap: 4px;
}

.status-pill.ok {
  background: rgba(46, 160, 67, 0.15);
  border-color: rgba(46, 160, 67, 0.4);
  color: #3fb950;
}

.status-pill.fail {
  background: rgba(248, 81, 73, 0.15);
  border-color: rgba(248, 81, 73, 0.4);
  color: #f85149;
}

.status-pill.untested {
  background: #21262d;
  color: #8b949e;
}

.status-pill.testing {
  background: rgba(56, 139, 253, 0.15);
  color: #58a6ff;
}

.node-actions {
  display: flex;
  gap: 4px;
}

.session-notice-box {
  margin-top: 10px;
  padding: 8px 12px;
  background: rgba(210, 153, 34, 0.12);
  border: 1px solid rgba(210, 153, 34, 0.35);
  border-radius: 6px;
  color: #e3b341;
  font-size: 12px;
  display: flex;
  align-items: center;
  gap: 8px;
}

/* PROJECT DIRECTORY CARD */
.project-input-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.input-wrap {
  flex: 1;
  position: relative;
  display: flex;
  align-items: center;
}

.input-icon {
  position: absolute;
  left: 12px;
  color: #8b949e;
}

.project-input-row .form-control {
  padding-left: 36px;
}

/* LAUNCH FOOTER */
.launch-card {
  background: linear-gradient(180deg, #161b22 0%, #1a202c 100%);
  border: 1px solid #388bfd44;
  border-radius: 10px;
  padding: 14px 18px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.launch-settings {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.terminal-selector {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.field-label {
  color: #8b949e;
}

.select-control {
  background: #0d1117;
  border: 1px solid #30363d;
  color: #e6edf3;
  padding: 5px 10px;
  border-radius: 6px;
  font-size: 13px;
  outline: none;
}

.active-summary {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: #8b949e;
}

.active-summary strong {
  color: #58a6ff;
}

.summary-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #3fb950;
}

.btn-launch {
  background: linear-gradient(135deg, #238636, #2ea043);
  color: #ffffff;
  border: none;
  font-size: 15px;
  font-weight: 600;
  padding: 10px 24px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  box-shadow: 0 4px 14px rgba(35, 134, 54, 0.4);
  transition: all 0.2s;
}

.btn-launch:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 6px 18px rgba(35, 134, 54, 0.6);
}

.btn-launch:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  box-shadow: none;
}

/* STATUS BAR */
.app-statusbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 14px;
  background: #0d1117;
  border-top: 1px solid #21262d;
  font-size: 11px;
  color: #8b949e;
}

.status-left {
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-value {
  color: #c9d1d9;
}

.status-separator {
  color: #30363d;
}

/* BUTTONS */
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border-radius: 6px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
  outline: none;
}

.btn-sm { padding: 5px 10px; font-size: 12px; }
.btn-xs { padding: 2px 8px; font-size: 11px; }

.btn-primary {
  background: #238636;
  color: #fff;
  border: 1px solid rgba(240, 246, 252, 0.1);
}
.btn-primary:hover { background: #2ea043; }

.btn-primary-soft {
  background: rgba(56, 139, 253, 0.15);
  color: #58a6ff;
  border: 1px solid rgba(56, 139, 253, 0.3);
}
.btn-primary-soft:hover {
  background: rgba(56, 139, 253, 0.25);
}

.btn-outline {
  background: #21262d;
  color: #c9d1d9;
  border: 1px solid #30363d;
}
.btn-outline:hover {
  background: #30363d;
  border-color: #8b949e;
}

.btn-ghost {
  background: transparent;
  color: #c9d1d9;
  border: 1px solid transparent;
}
.btn-ghost:hover {
  background: #21262d;
}

.text-danger { color: #f85149 !important; }
.text-warning { color: #d29922 !important; }
.text-sub { color: #8b949e; }

/* FORMS */
.form-group {
  margin-bottom: 12px;
}

.form-label {
  display: block;
  font-size: 12px;
  color: #8b949e;
  margin-bottom: 4px;
}

.form-control {
  width: 100%;
  background: #0d1117;
  border: 1px solid #30363d;
  color: #e6edf3;
  padding: 7px 10px;
  border-radius: 6px;
  font-size: 13px;
  outline: none;
  box-sizing: border-box;
}

.form-control:focus {
  border-color: #58a6ff;
}

.field-hint {
  font-size: 11px;
  color: #8b949e;
  margin: 4px 0 0 0;
}

.form-row {
  display: flex;
  gap: 12px;
}

.flex-1 { flex: 1; }

.form-checkbox {
  margin-top: 10px;
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #c9d1d9;
  cursor: pointer;
}

.slug-preview {
  display: flex;
  align-items: center;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  padding: 0 8px;
}

.slug-prefix, .slug-suffix {
  color: #8b949e;
  font-family: monospace;
}

.slug-input {
  border: none;
  background: transparent;
  color: #79c0ff;
  font-family: monospace;
  padding: 7px 4px;
}

.slug-input:focus {
  outline: none;
}

.eye-btn {
  position: absolute;
  right: 8px;
  background: none;
  border: none;
  color: #8b949e;
  cursor: pointer;
  font-size: 11px;
}

.section-divider {
  border-bottom: 1px solid #30363d;
  line-height: 0.1em;
  margin: 16px 0 14px 0;
  text-align: center;
}

.section-divider span {
  background: #161b22;
  padding: 0 10px;
  color: #8b949e;
  font-size: 11px;
}

/* MODALS */
.modal-backdrop {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(2px);
}

.modal-dialog {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  width: 480px;
  max-width: 90vw;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.6);
}

.modal-lg {
  width: 680px;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 18px;
  border-bottom: 1px solid #30363d;
}

.modal-title {
  margin: 0;
  font-size: 15px;
  color: #f0f6fc;
}

.modal-close {
  background: none;
  border: none;
  color: #8b949e;
  font-size: 20px;
  cursor: pointer;
}

.modal-body {
  padding: 16px 18px;
  overflow-y: auto;
  flex: 1;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 12px 18px;
  border-top: 1px solid #30363d;
}

.info-callout {
  background: #0d1117;
  border-left: 3px solid #58a6ff;
  padding: 10px 12px;
  border-radius: 4px;
  margin-top: 12px;
}

.info-title {
  font-size: 12px;
  font-weight: 600;
  color: #58a6ff;
  margin-bottom: 2px;
}

.info-desc {
  font-size: 11px;
  color: #8b949e;
  margin: 0;
}

/* DATA TABLE */
.data-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}

.data-table th, .data-table td {
  padding: 8px 10px;
  border-bottom: 1px solid #30363d;
  text-align: left;
}

.data-table th {
  color: #8b949e;
  font-weight: 500;
}

.table-actions {
  display: flex;
  gap: 6px;
}

/* DIFF PREVIEW */
.diff-summary-box {
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  padding: 10px 14px;
  margin-bottom: 12px;
}

.diff-summary-box h4 {
  margin: 0 0 6px 0;
  font-size: 13px;
  color: #58a6ff;
}

.diff-summary-box ul {
  margin: 0;
  padding-left: 18px;
  font-size: 12px;
  color: #c9d1d9;
}

.diff-file-tag {
  font-size: 11px;
  color: #8b949e;
  margin-bottom: 6px;
  font-family: monospace;
}

.json-code-box {
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  padding: 10px;
  max-height: 280px;
  overflow-y: auto;
}

.json-code-box pre {
  margin: 0;
  font-family: "Consolas", monospace;
  font-size: 12px;
  color: #79c0ff;
}

.code-textarea {
  width: 100%;
  background: #0d1117;
  border: 1px solid #30363d;
  color: #79c0ff;
  font-family: "Consolas", monospace;
  font-size: 12px;
  padding: 10px;
  border-radius: 6px;
  box-sizing: border-box;
  outline: none;
}

/* SETTINGS MODAL */
.settings-section {
  margin-bottom: 18px;
  border-bottom: 1px solid #21262d;
  padding-bottom: 14px;
}

.settings-section:last-child {
  border-bottom: none;
}

.section-title {
  margin: 0 0 8px 0;
  font-size: 14px;
  color: #f0f6fc;
}

.form-radio-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 8px;
}

.radio-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #c9d1d9;
  cursor: pointer;
}

.custom-dir-row {
  margin: 8px 0;
}

.dir-status-box {
  background: #0d1117;
  padding: 8px 10px;
  border-radius: 6px;
  font-size: 11px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.backups-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 180px;
  overflow-y: auto;
}

.backup-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: #0d1117;
  padding: 6px 10px;
  border-radius: 6px;
  border: 1px solid #30363d;
}

.backup-name {
  font-size: 12px;
  font-family: monospace;
  color: #c9d1d9;
}

.backup-date {
  font-size: 11px;
  color: #8b949e;
  margin-left: 8px;
}

.spinner-sm {
  display: inline-block;
  width: 10px;
  height: 10px;
  border: 2px solid #58a6ff;
  border-top-color: transparent;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
