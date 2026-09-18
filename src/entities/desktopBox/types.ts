/**
 * Box 标题位置由窗口菜单控制，每个 Box 可独立保存自己的布局偏好
 */
export type DesktopBoxTitlePosition = "top" | "bottom";

/**
 * Box 是一个真实收纳文件夹的桌面窗口，标题只用于展示，不参与真实文件夹命名
 */
export interface DesktopBox {
  /**
   * 收缩模式开启后，鼠标离开 Box 会折叠到只剩标题栏，hover 标题时临时展开
   */
  collapsed: boolean;
  /**
   * 业务 ID 同时用于窗口 label 和默认物理目录名的稳定片段
   */
  id: string;
  /**
   * 由 Dasktop 创建并长期绑定的真实文件夹路径，Box 内容由 Windows Explorer 原生视图展示
   */
  folderPath: string;
  /**
   * 锁定后禁止移动和改变窗口大小，但 Box 内文件操作仍然可用
   */
  locked: boolean;
  title: string;
  /**
   * Box 闲置可见度，0 表示鼠标未进入 Box 区域时整体透明；hover 后恢复完全可见
   */
  titleOpacity: number;
  /**
   * 标题位置属于单个 Box 的布局偏好，不参与全局设置同步
   */
  titlePosition: DesktopBoxTitlePosition;
  x: number;
  y: number;
  width: number;
  height: number;
  /**
   * Box 预设图标或封面标识，小图标模式或标题栏展示时优先使用该图标
   */
  icon?: string;
}

