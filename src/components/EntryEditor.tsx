import { useState, useEffect } from "react";
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

    useEffect(() => {
        if (selectedId !== null) {
          invoke<{ title: string; body: string }>("get_entry", { id: selectedId })
            .then((entry) => {
              setTitle(entry.title);
              setBody(entry.body);
            })
            .catch((err) => console.error("Load error:", err));
        } else {
          // Reset the editor when creating a new entry
          setTitle("");
          setBody("");
        }
      }, [selectedId]);
  
    const handleTitleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
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
            <input
              className="editor-title"
              type="text"
              placeholder="Title"
              value={title}
              onChange={handleTitleChange}
            />
          </label>
          <label className="editor-body-container">
            <textarea
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