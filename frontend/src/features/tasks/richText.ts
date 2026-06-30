import DOMPurify from "dompurify";
import { generateHTML } from "@tiptap/core";
import type { JSONContent } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import Link from "@tiptap/extension-link";
import Image from "@tiptap/extension-image";
import Underline from "@tiptap/extension-underline";
import Table from "@tiptap/extension-table";
import TableRow from "@tiptap/extension-table-row";
import TableCell from "@tiptap/extension-table-cell";
import TableHeader from "@tiptap/extension-table-header";
import TaskList from "@tiptap/extension-task-list";
import TaskItem from "@tiptap/extension-task-item";

const descriptionExtensions = [
  StarterKit.configure({
    heading: { levels: [1, 2, 3] },
  }),
  Underline,
  Link.configure({ openOnClick: false }),
  Image,
  Table.configure({ resizable: false }),
  TableRow,
  TableHeader,
  TableCell,
  TaskList,
  TaskItem.configure({ nested: true }),
];

export function renderDescriptionHtml(value: Record<string, unknown>): string {
  if (typeof value.html === "string") {
    return DOMPurify.sanitize(value.html);
  }
  if (value.type === "doc" && Array.isArray(value.content) && value.content.length > 0) {
    try {
      return DOMPurify.sanitize(generateHTML(value as JSONContent, descriptionExtensions));
    } catch {
      return "";
    }
  }
  return "";
}
