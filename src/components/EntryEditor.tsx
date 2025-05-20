import React from 'react';
import { useState, useEffect, useRef } from "react";
import "./EntryEditor.css";
import { invoke } from "@tauri-apps/api/core";
import { EditorContent, EditorContext, useEditor} from '@tiptap/react'
import Heading from '@tiptap/extension-heading';
import StarterKit from '@tiptap/starter-kit';
import { Markdown }  from 'tiptap-markdown'
import Link from '@tiptap/extension-link';
import { Extension } from '@tiptap/core';
import { Plugin } from 'prosemirror-state';
import { openUrl } from '@tauri-apps/plugin-opener';
import { readText } from '@tauri-apps/plugin-clipboard-manager';

const PasteLinkOnSelection = Extension.create({
  name: 'pasteLinkOnSelection',
  addProseMirrorPlugins() {
    return [
      new Plugin({
        props: {
          handlePaste(view, event) {
            const text = event.clipboardData?.getData('text/plain');
            const { empty } = view.state.selection;
            if (text?.match(/^https?:\/\//) && !empty) {
              const { state, dispatch } = view;
              const { from, to } = state.selection;
              const linkMark = state.schema.marks.link.create({ href: text });
              const tr = state.tr.addMark(from, to, linkMark);
              dispatch(tr);
              return true;
            }
            return false;
          },
        },
      }),
    ];
  },
});

type Props = {
    selectedId: number | null;
    refreshEntries: () => void;
    updateEntryTitle: (id: number, title: string) => void;
};

export default function EntryEditor({ selectedId, refreshEntries, updateEntryTitle }: Props) {
    const [title, setTitle] = useState("");
    const [body, setBody] = useState("");
    const [createdAt, setCreatedAt] = useState<string | null>(null);

    const titleRef = useRef<HTMLTextAreaElement>(null);

    const editor = useEditor({
      immediatelyRender: false,
      extensions: [
        StarterKit.configure({ heading: false }),
        Link.configure({
          autolink: true,
          linkOnPaste: true,
          openOnClick: true,
        }),
        PasteLinkOnSelection,
        Heading.configure({ levels: [1, 2] }),
        Markdown.configure({              
          html:       true,               
          tightLists: true,                
          bulletListMarker: '-',          
        }),
      ],
      content: '',
      onUpdate({ editor }) {
        const html = editor.getHTML();
        setBody(html);
      },
    });

    useEffect(() => {
      if (!editor) return;

      if (selectedId !== null) {
        invoke<{ title: string; body: string; created_at: string }>("get_entry", { id: selectedId })
          .then((entry) => {
            setTitle(entry.title);
            setBody(entry.body || "");
            setCreatedAt(entry.created_at);
            editor.commands.setContent(entry.body || '');
            requestAnimationFrame(() => {
              if (entry.title.trim().length === 0) {
                titleRef.current?.focus();
              } else {
                editor.commands.focus("end");
              }
            });
          })
          .catch((err) => console.error("Load error:", err));
      } else {
        setTitle("");
        setBody("");
        setCreatedAt(null);
        editor.commands.clearContent();
        requestAnimationFrame(() => {
          titleRef.current?.focus();
        });
      }
    }, [selectedId, editor]);
  // Ensure title textarea grows to fit initial content
  useEffect(() => {
    if (titleRef.current) {
      titleRef.current.style.height = 'auto';
      titleRef.current.style.height = `${titleRef.current.scrollHeight}px`;
    }
  }, [title]);
  
    useEffect(() => {
      const timeout = setTimeout(() => {
        if (selectedId !== null && (title.trim() || body.trim())) {
          invoke("save_entry", { id: selectedId, title, body })
            .then(() => {
              console.log("Autosaved");
              if (title.trim()) {
                refreshEntries();
              }
            })
            .catch((err) => console.error("Save error:", err));
        }
      }, 1000); // autosave after 1s of pause
  
      return () => clearTimeout(timeout);
    }, [title, body, selectedId, refreshEntries]);
  
    const handleTitleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
     // Auto-resize textarea height
     e.currentTarget.style.height = 'auto';
     e.currentTarget.style.height = `${e.currentTarget.scrollHeight}px`;
     const newTitle = e.currentTarget.value;
     setTitle(newTitle);
     if (selectedId !== null) {
       updateEntryTitle(selectedId, newTitle);
     }
   };

    const handleKeyDown = (e: React.KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      switch (e.key.toLowerCase()) {
        case 'x':
          document.execCommand('cut');
          e.preventDefault();
          break;
        case 'c':
          document.execCommand('copy');
          e.preventDefault();
          break;
        case 'v': {
          e.preventDefault();
          readText().then((text) => {
            if (!editor) return;
            const { empty } = editor.state.selection;
            if (!empty && /^https?:\/\//.test(text)) {
              editor.chain().focus().extendMarkRange('link').setLink({ href: text }).run();
            } else {
              editor.chain().focus().insertContent(text).run();
            }
          });
          break;
        }
        case 'z':
          document.execCommand(e.shiftKey ? 'redo' : 'undo');
          e.preventDefault();
          break;
        default:
          break;
      }
    };

    const handleLinkClick = (e: React.MouseEvent) => {
      const target = e.target as HTMLElement;
      if (target.tagName === 'A') {
        e.preventDefault();
        const href = (target as HTMLAnchorElement).href;
        if (href) {
          openUrl(href);
        }
      }
    };
  
    return (
      <>
        <form className="editor " onSubmit={(e) => e.preventDefault()}>
        {createdAt && (
              <time
                dateTime={createdAt}
                className="text-sm text-gray-500 mt-1 block"
              >
                {new Date(createdAt).toLocaleDateString()}
              </time>
            )}
          <label>
            <textarea
              ref={titleRef}
              className="editor-title font-serif font-bold text-2xl resize-none overflow-hidden"
              placeholder="Title"
              value={title}
              onInput={handleTitleChange}
              rows={1}
              style={{ height: 'auto' }}
            />
          </label>
          <label
            className="editor-body-container font-sans"
            style={{ flex: 1 }}
          >
            <EditorContext.Provider value={{ editor }}>
              <EditorContent
                editor={editor}
                onKeyDown={handleKeyDown}
                className="editor-body"
                onClick={(e) => {
                  editor?.commands.focus();
                  handleLinkClick(e);
                }}
                onPaste={(event) => {
                  const text = event.clipboardData.getData('text/plain');
                  const urlPattern = /^https?:\/\/\S+$/;
                  if (editor && urlPattern.test(text) && !editor.state.selection.empty) {
                    event.preventDefault();
                    editor.chain().focus().extendMarkRange('link').setLink({ href: text }).run();
                  }
                }}
                style={{
                  flex: 1,
                  border: "none",
                  outline: "none",
                  boxShadow: "none",
                  width: "100%",
                  resize: "none",
                  padding: "0",
                  boxSizing: "border-box",
                  overflow: "auto",
                  caretColor: "auto",
                  userSelect: "text"
                }}
              />
            </EditorContext.Provider>
          </label>
        </form>
      </>
    );
}