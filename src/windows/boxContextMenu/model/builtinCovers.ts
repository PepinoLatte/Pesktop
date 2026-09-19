import type { Component } from "vue";
import {
  Bookmark,
  Briefcase,
  Calendar,
  Cloud,
  Download,
  FileText,
  Flag,
  Folder,
  Gamepad2,
  Heart,
  Image,
  Lightbulb,
  Lock,
  Mail,
  Music,
  Star,
} from "@lucide/vue";

/**
 * 内置封面图标的统一渲染元信息；id 落库为 `builtin:<id>`，组件供菜单选择器和
 * Box 图标态入口共同消费，保证选择与展示用同一套视觉
 */
export interface BuiltinCoverIcon {
  component: Component;
  id: string;
  label: string;
}

/**
 * 内置图标库：图标组件来自应用既有依赖，新增封面只需要在表内补一行
 */
export const BUILTIN_COVER_ICONS: BuiltinCoverIcon[] = [
  { component: Folder, id: "folder", label: "文件夹" },
  { component: Star, id: "star", label: "星标" },
  { component: Heart, id: "heart", label: "收藏" },
  { component: Briefcase, id: "briefcase", label: "工作" },
  { component: Gamepad2, id: "gamepad", label: "游戏" },
  { component: Music, id: "music", label: "音乐" },
  { component: Image, id: "image", label: "图片" },
  { component: Download, id: "download", label: "下载" },
  { component: FileText, id: "file-text", label: "文档" },
  { component: Calendar, id: "calendar", label: "日程" },
  { component: Mail, id: "mail", label: "邮件" },
  { component: Cloud, id: "cloud", label: "云端" },
  { component: Lock, id: "lock", label: "私密" },
  { component: Lightbulb, id: "lightbulb", label: "灵感" },
  { component: Bookmark, id: "bookmark", label: "书签" },
  { component: Flag, id: "flag", label: "标记" },
];

/**
 * 内置封面在数据库中的完整存储值
 */
export function builtinCoverValue(id: string): string {
  return `builtin:${id}`;
}

/**
 * 解析封面存储值：`builtin:` 前缀返回内置图标 id，`data:image/` 返回 null 由图片分支处理
 */
export function parseBuiltinCoverId(coverIcon: string | null): string | null {
  if (!coverIcon?.startsWith("builtin:")) {
    return null;
  }

  return coverIcon.slice("builtin:".length);
}
