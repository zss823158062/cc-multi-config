export interface Group {
  id: string;
  name: string;
  settingsFile: string;
  activeNodeId: string | null;
  defaultNodeId: string | null;
}

export interface NodeModels {
  opus?: string;
  sonnet?: string;
  haiku?: string;
  defaultModel?: string;
}

export interface RelayNode {
  id: string;
  name: string;
  baseUrl: string;
  authMode: 'authToken' | 'apiKey';
  authToken: string;
  models: NodeModels;
  customEnv?: Record<string, string>;
}

export interface Preferences {
  configDirMode: 'auto' | 'custom';
  customClaudeConfigDir: string | null;
  terminal: string;
  claudeExecutablePath: string | null;
  lastProjectDir: string | null;
}

export interface LauncherConfig {
  schemaVersion: number;
  groups: Group[];
  preferences: Preferences;
}

export interface NodeTestResult {
  nodeId: string;
  connectivityOk: boolean;
  latencyMs?: number;
  statusCode?: number;
  modelOk?: boolean;
  message: string;
}

export interface BackupItem {
  filename: string;
  groupId: string;
  timestamp: string;
  sizeBytes: number;
  fullPath: string;
}

export interface BaseSettingsStatus {
  exists: boolean;
  readable: boolean;
  path: string;
  error?: string;
}

export interface ConfigDirInfo {
  mode: string;
  resolvedPath: string;
  isValid: boolean;
  hasBaseSettings: boolean;
  baseSettingsPath: string;
  message?: string;
}

export interface ClaudeBinaryInfo {
  path: string;
  isDetected: boolean;
  version?: string;
  source: string;
}

export interface TerminalInfo {
  id: string;
  name: string;
  isAvailable: boolean;
  isDefault: boolean;
}

export interface SettingsDiffResult {
  groupId: string;
  currentContent: string;
  previewContent: string;
  diffSummary: string[];
  targetFilePath: string;
}

export interface LaunchResult {
  success: boolean;
  message: string;
  terminalUsed: string;
  command: string;
}
