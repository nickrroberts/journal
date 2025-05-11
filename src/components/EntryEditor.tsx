import { useState, useEffect, useRef } from "react";
import "./EntryEditor.css";
import { invoke } from "@tauri-apps/api/core";
import { EditorContent, EditorContext, useEditor} from '@tiptap/react'
import Heading from '@tiptap/extension-heading';
import StarterKit from '@tiptap/starter-kit';
import { Markdown }  from 'tiptap-markdown'

type Props = {
    selectedId: number | null;
    refreshEntries: () => void;
    updateEntryTitle: (id: number, title: string) => void;
};

export default function EntryEditor({ selectedId, refreshEntries, updateEntryTitle }: Props) {
    const [title, setTitle] = useState("");
    const [body, setBody] = useState("");

    const titleRef = useRef<HTMLTextAreaElement>(null);

    const editor = useEditor({
      immediatelyRender: false,
      extensions: [
        StarterKit.configure({ heading: false }),
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
        invoke<{ title: string; body: string }>("get_entry", { id: selectedId })
          .then((entry) => {
            setTitle(entry.title);
            setBody(entry.body || "");
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
        editor.commands.clearContent();
        requestAnimationFrame(() => {
          titleRef.current?.focus();
        });
      }
    }, [selectedId, editor]);
  
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
      const newTitle = e.target.value;
      setTitle(newTitle);
      if (selectedId !== null) {
        updateEntryTitle(selectedId, newTitle);
      }
    };


  
    return (
      <>
        <form className="editor" onSubmit={(e) => e.preventDefault()}>
          <label>
            <textarea
              ref={titleRef}
              className="editor-title"
              placeholder="Title"
              value={title}
              onChange={handleTitleChange}
              rows={1}
            />
          </label>
          <label className="editor-body-container">
            <EditorContext.Provider value={{ editor }}>
              <EditorContent
                editor={editor}
                className="editor-body"
                onClick={() => editor?.commands.focus()}
                style={{
                  flex: 1,
                  border: "none",
                  outline: "none",
                  boxShadow: "none",
                  width: "100%",
                  height: "100%",
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