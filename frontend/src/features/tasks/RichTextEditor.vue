<script setup lang="ts">
import { watch } from 'vue';
import { EditorContent, useEditor } from '@tiptap/vue-3';
import StarterKit from '@tiptap/starter-kit';
import { Bold, Italic, List, ListOrdered } from 'lucide-vue-next';

const props = withDefaults(
  defineProps<{
    modelValue?: unknown;
    disabled?: boolean;
  }>(),
  {
    modelValue: undefined,
    disabled: false,
  },
);

const emit = defineEmits<{
  'update:modelValue': [value: unknown];
}>();

const editor = useEditor({
  extensions: [StarterKit],
  content: props.modelValue ?? emptyDescriptionDoc(),
  editable: !props.disabled,
  editorProps: {
    attributes: {
      class: 'rich-text-editor__content-area',
    },
  },
  onUpdate({ editor }) {
    emit('update:modelValue', editor.getJSON());
  },
});

watch(
  () => props.modelValue,
  (value) => {
    const instance = editor.value;
    if (!instance) {
      return;
    }

    const nextValue = value ?? emptyDescriptionDoc();
    if (JSON.stringify(instance.getJSON()) !== JSON.stringify(nextValue)) {
      instance.commands.setContent(nextValue, { emitUpdate: false });
    }
  },
);

watch(
  () => props.disabled,
  (disabled) => {
    editor.value?.setEditable(!disabled);
  },
);

function toggleBold() {
  editor.value?.chain().focus().toggleBold().run();
}

function toggleItalic() {
  editor.value?.chain().focus().toggleItalic().run();
}

function toggleBulletList() {
  editor.value?.chain().focus().toggleBulletList().run();
}

function toggleOrderedList() {
  editor.value?.chain().focus().toggleOrderedList().run();
}

function emptyDescriptionDoc() {
  return {
    type: 'doc',
    content: [{ type: 'paragraph', content: [] }],
  };
}
</script>

<template>
  <div class="rich-text-editor">
    <div class="rich-text-editor__toolbar">
      <button
        type="button"
        class="rich-text-editor__tool"
        :class="{ 'rich-text-editor__tool--active': editor?.isActive('bold') }"
        :disabled="disabled"
        title="加粗"
        @click="toggleBold"
      >
        <Bold :size="16" />
      </button>
      <button
        type="button"
        class="rich-text-editor__tool"
        :class="{ 'rich-text-editor__tool--active': editor?.isActive('italic') }"
        :disabled="disabled"
        title="斜体"
        @click="toggleItalic"
      >
        <Italic :size="16" />
      </button>
      <button
        type="button"
        class="rich-text-editor__tool"
        :class="{ 'rich-text-editor__tool--active': editor?.isActive('bulletList') }"
        :disabled="disabled"
        title="无序列表"
        @click="toggleBulletList"
      >
        <List :size="16" />
      </button>
      <button
        type="button"
        class="rich-text-editor__tool"
        :class="{ 'rich-text-editor__tool--active': editor?.isActive('orderedList') }"
        :disabled="disabled"
        title="有序列表"
        @click="toggleOrderedList"
      >
        <ListOrdered :size="16" />
      </button>
    </div>
    <EditorContent :editor="editor" />
  </div>
</template>
