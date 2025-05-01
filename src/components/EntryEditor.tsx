import { useState, useEffect, useRef } from "react";
import "./EntryEditor.css";
import { invoke } from "@tauri-apps/api/core";

type Props = {
    selectedId: number | null;
    refreshEntries: () => void;
    updateEntryTitle: (id: number, title: string) => void;
};

export default function EntryEditor({ selectedId, refreshEntries, updateEntryTitle }: Props) {
    const [title, setTitle] = useState("");
    const [body, setBody] = useState("");

    const titleRef = useRef<HTMLTextAreaElement>(null);
    const bodyRef = useRef<HTMLTextAreaElement>(null);

    useEffect(() => {
        if (selectedId !== null) {
          invoke<{ title: string; body: string }>("get_entry", { id: selectedId })
            .then((entry) => {
              setTitle(entry.title);
              setBody(entry.body);
              requestAnimationFrame(() => {
                if (entry.title.trim().length === 0) {
                  titleRef.current?.focus();
                } else {
                  bodyRef.current?.focus();
                }
              });
            })
            .catch((err) => console.error("Load error:", err));
        } else {
          // Reset the editor when creating a new entry
          setTitle("");
          setBody("");
          requestAnimationFrame(() => {
            titleRef.current?.focus();
          });
        }
      }, [selectedId]);
  
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
            <textarea
              ref={bodyRef}
              className="editor-body"
              placeholder="Write your journal entry..."
              value={body}
              onChange={(e) => setBody(e.target.value)}
            />
          </label>
        </form>
      </>
    );
}