import { invoke } from '@tauri-apps/api/core';
import type {
  LauncherConfig,
  Group,
  RelayNode,
  Preferences,
  NodeTestResult,
  BackupItem,
  BaseSettingsStatus,
  ConfigDirInfo,
  ClaudeBinaryInfo,
  TerminalInfo,
  SettingsDiffResult,
  LaunchResult,
} from './types';

export const api = {
  getLauncherConfig: () => invoke<LauncherConfig>('get_launcher_config'),
  getGroups: () => invoke<Group[]>('get_groups'),
  createGroup: (name: string, slug?: string, initialNodeId?: string) =>
    invoke<Group>('create_group', { name, slug, initialNodeId }),
  deleteGroup: (groupId: string, deleteFile: boolean) =>
    invoke<void>('delete_group', { groupId, deleteFile }),

  getNodes: () => invoke<RelayNode[]>('get_nodes'),
  saveNode: (node: RelayNode, applyToGroups: boolean) =>
    invoke<RelayNode>('save_node', { node, applyToGroups }),
  deleteNode: (nodeId: string) =>
    invoke<void>('delete_node', { nodeId }),
  testNode: (node: RelayNode, testModel: boolean) =>
    invoke<NodeTestResult>('test_node', { node, testModel }),
  testAllNodes: (testModel: boolean) =>
    invoke<NodeTestResult[]>('test_all_nodes', { testModel }),

  switchGroupNode: (groupId: string, nodeId: string) =>
    invoke<void>('switch_group_node', { groupId, nodeId }),
  getGroupSettingsContent: (groupId: string) =>
    invoke<string>('get_group_settings_content', { groupId }),
  saveGroupSettingsContent: (groupId: string, content: string) =>
    invoke<void>('save_group_settings_content', { groupId, content }),
  previewSettingsDiff: (groupId: string, nodeId: string) =>
    invoke<SettingsDiffResult>('preview_settings_diff', { groupId, nodeId }),

  getClaudeConfigDirInfo: () =>
    invoke<ConfigDirInfo>('get_claude_config_dir_info'),
  checkBaseSettings: () =>
    invoke<BaseSettingsStatus>('check_base_settings'),
  createEmptyBaseSettings: () =>
    invoke<string>('create_empty_base_settings'),

  getPreferences: () => invoke<Preferences>('get_preferences'),
  savePreferences: (preferences: Preferences) =>
    invoke<void>('save_preferences', { preferences }),

  detectClaudeBinary: (customPath?: string) =>
    invoke<ClaudeBinaryInfo>('detect_claude_binary', { customPath }),
  getAvailableTerminals: () =>
    invoke<TerminalInfo[]>('get_available_terminals'),

  listBackups: (groupId?: string) =>
    invoke<BackupItem[]>('list_backups', { groupId }),
  restoreBackup: (backupFilename: string, groupId: string) =>
    invoke<void>('restore_backup', { backupFilename, groupId }),

  openInExplorer: (path: string) =>
    invoke<void>('open_in_explorer', { path }),
  openFileInEditor: (path: string) =>
    invoke<void>('open_file_in_editor', { path }),
  selectFolder: (defaultPath?: string) =>
    invoke<string | null>('select_folder', { defaultPath }),

  launchClaude: (groupId: string, projectDir: string, terminalOverride?: string) =>
    invoke<LaunchResult>('launch_claude', { groupId, projectDir, terminalOverride }),

  listConfigFiles: () =>
    invoke<import('./types').ConfigFileItem[]>('list_config_files'),

  getOsInfo: () =>
    invoke<import('./types').OsInfo>('get_os_info'),
};
