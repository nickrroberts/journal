import { useState, useEffect, useRef } from "react";
import EntryEditor from "./components/EntryEditor";
import EntryList from "./components/EntryList";
import { invoke } from "@tauri-apps/api/core";

type Entry = {
  id: number;
  title: string;
  created_at: string;
};

export default function App() {
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [entries, setEntries] = useState<Entry[]>([]);
  const [isBlurred, setIsBlurred] = useState(false);
  const inactivityTimer = useRef<NodeJS.Timeout | null>(null);
  const INACTIVITY_DURATION = 60000; // blur after a min

  const refreshEntries = () => {
    invoke<Entry[]>("get_entries")
      .then(setEntries)
      .catch((err) => console.error("Failed to fetch entries:", err));
  };

  const updateEntryTitle = (id: number, newTitle: string) => {
    setEntries(prevEntries => 
      prevEntries.map(entry => 
        entry.id === id ? { ...entry, title: newTitle } : entry
      )
    );
  };

  const resetInactivityTimer = () => {
    if (inactivityTimer.current) {
      clearTimeout(inactivityTimer.current);
    }
    inactivityTimer.current = setTimeout(() => {
      setIsBlurred(true);
    }, INACTIVITY_DURATION);
  };

  const handleUserActivity = () => {
    resetInactivityTimer();
  };

  const handleClick = (e: React.MouseEvent) => {
    if (isBlurred) {
      setIsBlurred(false);
      resetInactivityTimer();
    }
  };

  useEffect(() => {
    refreshEntries();
  }, []);

  useEffect(() => {
    // Set up initial timer
    resetInactivityTimer();

    // Add event listeners for user activity
    window.addEventListener('mousemove', handleUserActivity);
    window.addEventListener('keydown', handleUserActivity);
    window.addEventListener('click', handleUserActivity);
    window.addEventListener('scroll', handleUserActivity);

    // Cleanup
    return () => {
      if (inactivityTimer.current) {
        clearTimeout(inactivityTimer.current);
      }
      window.removeEventListener('mousemove', handleUserActivity);
      window.removeEventListener('keydown', handleUserActivity);
      window.removeEventListener('click', handleUserActivity);
      window.removeEventListener('scroll', handleUserActivity);
    };
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === 'b') {
        setIsBlurred(prev => !prev);
        resetInactivityTimer();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  return (
    <div 
      onClick={handleClick}
      style={{ 
        display: "flex", 
        height: "100vh", 
        margin: 0, 
        padding: 0, 
        overflow: "hidden",
        filter: isBlurred ? "blur(8px)" : "none",
        transition: "filter 0.3s ease",
        cursor: isBlurred ? "pointer" : "default"
      }}
    >
      <div style={{ 
        width: "250px", 
        background: "#f2f2f2", 
        margin: 0, 
        padding: 0, 
        height: "100vh", 
        display: "flex",
        flexDirection: "column"
      }}>
        <EntryList 
          entries={entries}
          onSelect={setSelectedId} 
          activeId={selectedId}
          refreshEntries={refreshEntries}
          updateEntryTitle={updateEntryTitle}
        />
      </div>

      <div style={{ 
        flex: 1, 
        margin: 0, 
        padding: 0, 
        height: "100vh", 
        overflow: "hidden"
      }}>
        <EntryEditor 
          selectedId={selectedId} 
          refreshEntries={refreshEntries}
          updateEntryTitle={updateEntryTitle}
        />
      </div>
    </div>
  );
}
