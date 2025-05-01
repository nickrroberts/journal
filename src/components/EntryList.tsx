import { invoke } from "@tauri-apps/api/core";

type Entry = {
  id: number;
  title: string;
  created_at: string;
};

type Props = {
  entries: Entry[];
  onSelect: (id: number | null) => void;
  activeId: number | null;
  refreshEntries: () => void;
  updateEntryTitle: (id: number, title: string) => void;
};

export default function EntryList({ 
  entries,
  onSelect, 
  activeId, 
  refreshEntries,
  updateEntryTitle
}: Props) {
  const createNewEntry = () => {
    invoke<number>("create_entry")
      .then((id) => {
        refreshEntries();
        onSelect(id);
      })
      .catch((err) => console.error("New entry error:", err));
  };

  return (
    <div style={{ padding: "1rem" }}>
      <button
        onClick={createNewEntry}
        style={{
          marginBottom: "1rem",
          padding: "0.5rem 1rem",
          fontSize: "1rem",
          cursor: "pointer",
        }}
      >
        + New Entry
      </button>
      <ul style={{ listStyle: "none", padding: 0 }}>
        {entries.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()).map((entry) => (
          <li key={entry.id}>
            <button
              onClick={() => onSelect(entry.id)}
              style={{
                display: "block",
                width: "100%",
                textAlign: "left",
                background: activeId === entry.id ? "#ddd" : "none",
                border: "none",
                padding: "0.5rem 0.75rem",
                cursor: "pointer",
                fontWeight: 500,
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis"
              }}
            >
              {entry.title || "Untitled"}
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}