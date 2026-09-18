/**
 * Box 预设图标配置与元数据定义
 * 供小图标模式、Box 标题栏以及右键菜单预选图标库使用
 */

/**
 * 预设图标项定义
 */
export interface BoxPresetIcon {
  /**
   * 图标唯一标识，与 Lucide 图标组件名对齐
   */
  id: string;
  /**
   * 规范展示名称
   */
  name: string;
  /**
   * 友好中文描述
   */
  label: string;
}

/**
 * 内置优质预选图标库列表
 * 涵盖工作、娱乐、多媒体、工具等核心桌面使用场景
 */
export const BOX_PRESET_ICONS: readonly BoxPresetIcon[] = [
  { id: "Folder", name: "Folder", label: "默认文件夹" },
  { id: "FolderArchive", name: "FolderArchive", label: "归档资产" },
  { id: "Code2", name: "Code2", label: "编程代码" },
  { id: "Gamepad2", name: "Gamepad2", label: "游戏娱乐" },
  { id: "Palette", name: "Palette", label: "设计创意" },
  { id: "Music", name: "Music", label: "音乐音频" },
  { id: "Film", name: "Film", label: "影视媒体" },
  { id: "FileText", name: "FileText", label: "文档笔记" },
  { id: "Briefcase", name: "Briefcase", label: "办公业务" },
  { id: "Terminal", name: "Terminal", label: "终端开发" },
  { id: "Sparkles", name: "Sparkles", label: "精选常用" },
  { id: "Star", name: "Star", label: "重点星标" },
  { id: "Rocket", name: "Rocket", label: "启动聚合" },
  { id: "Download", name: "Download", label: "下载缓存" },
  { id: "Monitor", name: "Monitor", label: "桌面硬件" },
  { id: "Coffee", name: "Coffee", label: "生活日常" },
  { id: "Layers", name: "Layers", label: "综合分组" },
  { id: "Compass", name: "Compass", label: "探索导航" },
] as const;
