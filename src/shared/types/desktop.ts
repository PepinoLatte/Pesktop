/**
 * 桌面文件类型由后端扫描真实桌面目录得到，前端只负责展示与分组映射。
 */
export type DesktopItemKind = "file" | "folder" | "shortcut" | "unknown";

/**
 * Box 图标名称显示模式由全局设置控制，避免不同窗口之间出现同一文件命名规则不一致。
 */
export type DesktopNameDisplayMode = "full" | "hideShortcutExtension" | "hideAllExtensions";

/**
 * Box 标题位置由窗口菜单控制，每个 Box 可独立保存自己的布局偏好。
 */
export type DesktopBoxTitlePosition = "top" | "bottom";

/**
 * Box 内拖拽排序的插入方向，用于在目标图标左右两侧显示位置提示。
 */
export type DesktopBoxItemDropPlacement = "before" | "after";

/**
 * 自绘桌面图标模型，path 是所有持久化映射的稳定主键。
 */
export interface DesktopItem {
  id: string;
  name: string;
  path: string;
  extension: string | null;
  kind: DesktopItemKind;
  /**
   * Windows Shell 解析出的原生图标，Box 和桌面列表复用同一份数据以避免拖入后视觉不一致。
   */
  iconDataUrl: string | null;
}

/**
 * Box 是桌面扩展层中的可视分组，不移动真实文件。
 */
export interface DesktopBox {
  /**
   * 收缩模式开启后，鼠标离开 Box 会折叠到只剩标题栏，hover 标题时临时展开。
   */
  collapsed: boolean;
  id: string;
  /**
   * 锁定后禁止移动和改变窗口大小，但 Box 内文件操作仍然可用。
   */
  locked: boolean;
  title: string;
  /**
   * Box 闲置可见度，0 表示鼠标未进入 Box 区域时整体透明；hover 后恢复完全可见。
   */
  titleOpacity: number;
  /**
   * 标题位置属于单个 Box 的布局偏好，不参与全局设置同步。
   */
  titlePosition: DesktopBoxTitlePosition;
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Box 与桌面项目的关联独立建模，便于按 Box 或路径快速查询、删除和去重。
 */
export interface DesktopBoxItem {
  boxId: string;
  itemPath: string;
  /**
   * Box 内的手动排序序号，值越小越靠前。
   */
  orderIndex: number;
}

/**
 * 桌面快照是后端扫描结果，前端基于它刷新 Box 可用项目。
 */
export interface DesktopSnapshot {
  desktopPath: string;
  items: DesktopItem[];
}

/**
 * 设置页只保留浅色、跟随系统和深色三种主题，避免与当前 UI 产生不一致的旧状态。
 */
export type ThemeMode = "light" | "system" | "dark";

/**
 * 应用设置只保存当前版本真实使用的偏好，旧外观参数不再兼容。
 */
export interface AppSettings {
  /**
   * 设置窗口独立使用的明暗主题，不再影响桌面上的 Box 窗口。
   */
  settingsTheme: ThemeMode;
  /**
   * Box 窗口独立使用的明暗主题，便于整理面板和设置页采用不同外观。
   */
  boxTheme: ThemeMode;
  snapToEdges: boolean;
  snapThreshold: number;
  /**
   * 是否给 Windows 桌面目录项目写入隐藏属性，从系统桌面隐藏原生图标。
   */
  nativeDesktopIconsHidden: boolean;
  /**
   * 隐藏原生桌面图标时仍保留显示的桌面项目路径。
   */
  nativeDesktopIconIgnorePaths: string[];
  showItemLabels: boolean;
  showShortcutArrow: boolean;
  doubleClickOpenItems: boolean;
  nameDisplayMode: DesktopNameDisplayMode;
  /**
   * Box 背景透明度使用百分比保存，便于设置页直接用滑块表达。
   */
  boxBackgroundOpacity: number;
  /**
   * Box 收缩和展开动画的持续时间，数值越小动画速度越快。
   */
  boxCollapseAnimationMs: number;
  /**
   * Box 内系统图标的最大显示尺寸，真实图像仍由 Windows Shell 解析。
   */
  boxIconSize: number;
  /**
   * Box 内文件名的字号，和图标尺寸独立配置以适配不同桌面密度。
   */
  boxLabelTextSize: number;
  /**
   * Box 图标列之间的横向间距，控制整理面板的信息密度。
   */
  boxIconGapX: number;
  /**
   * Box 图标行之间的纵向间距，避免长文件名换行后互相遮挡。
   */
  boxIconGapY: number;
  /**
   * Box 文件名的换行宽度，不改变真实文件名。
   */
  boxFilenameWidth: number;
  /**
   * Box 面板和图标命中区域的圆角，保持桌面组件视觉一致。
   */
  boxCornerRadius: number;
}
