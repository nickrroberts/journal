import * as Tooltip from '@radix-ui/react-tooltip';
import { useState } from 'react';
import { EllipsisVertical} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

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
  refreshEntries
}: Props) {
    const [menuForId, setMenuForId] = useState<number | null>(null);

    const showMenuFor = (id: number) => {
      setMenuForId(prev => (prev === id ? null : id));
    };

    const handleDelete = async (id: number) => {
      try {
        await invoke('delete_entry', { id });
        refreshEntries();
        if (activeId === id) onSelect(null);
        setMenuForId(null);
      } catch (err) {
        console.error('Delete entry error:', err);
      }
    };
  
  return (
    <div 
      style={{ 
        padding: "0 1.5rem 0 1rem",
        position: "relative",
        transition: "width 0.3s ease",
        overflow: "hidden"
      }}
    >
      <ul style={{ 
        listStyle: "none", 
        padding: 0,
        transition: "opacity 0.2s ease",
        marginTop: "2rem"
      }}>
        {entries.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()).map((entry) => (
          <li key={entry.id} className="entry-item">
            <button
              onClick={() => onSelect(entry.id)}
              style={{
                display: "block",
                width: "100%",
                textAlign: "left",
                background: activeId === entry.id ? "var(--entry-item-selected)" : "none",
                borderRadius: "5px",
                color: activeId === entry.id ? "var(--entry-item-selected-font)" : "",
                borderLeft: "4px solid",
                borderLeftColor: activeId === entry.id ? "#0070f3" : "transparent",
                padding: "0.5rem 0.75rem",
                cursor: "pointer",
                fontWeight: 500,
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis"
              }}
            >
              {entry.title || "Untitled"}
              <time
              dateTime={entry.created_at}
              className="text-xs text-gray-500 block"
              >
                {new Date(entry.created_at).toLocaleDateString()}
              </time>
            </button>
              <Tooltip.Root
                open={menuForId === entry.id}
                onOpenChange={(open) => {
                  if (!open) setMenuForId(null);
                }}
              >
                <Tooltip.Trigger asChild>
                  <EllipsisVertical 
                    size={20}
                    className="icon entry-menu" 
                    onClick={() => showMenuFor(entry.id)} 
                  />
                </Tooltip.Trigger>
                <Tooltip.Portal>
                  <Tooltip.Content 
                    side="right" 
                    align="center" 
                    sideOffset={5} 
                    className="tooltip-content"
                  >
                    <button
                      className="delete-entry-button"
                      onClick={() => handleDelete(entry.id)}
                    >
                      Delete
                    </button>
                  </Tooltip.Content>
                </Tooltip.Portal>
              </Tooltip.Root>
            
          </li>
        ))}
      </ul>
    </div>
  );
}