import { useState, useEffect, useRef } from "react";
import { getVersion } from "@tauri-apps/api/app";
import EntryEditor from "./components/EntryEditor";
import EntryList from "./components/EntryList";
import Settings from "./components/Settings";
import { invoke } from "@tauri-apps/api/core";
import { NotebookPen, Cog } from 'lucide-react';
import { createNewEntry } from './lib/createEntry';
import { X } from 'lucide-react';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { listen } from '@tauri-apps/api/event';
import changelog from "../changelog.json";
import Modal from "./components/Modal";

type Changelog = Record<string, string[]>;

type Entry = {
  id: number;
  title: string;
  created_at: string;
};

type Theme = 'system' | 'light' | 'dark';

export default function App() {
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [entries, setEntries] = useState<Entry[]>([]);
  const [isBlurred, setIsBlurred] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [theme, setTheme] = useState<Theme>('system');
  const [showChangelog, setShowChangelog] = useState(false);
  const [appVersion, setAppVersion] = useState<string>("");
  const inactivityTimer = useRef<NodeJS.Timeout | null>(null);
  const INACTIVITY_DURATION = 60000; // 1 minute of inactivity

  const [showUpToDate, setShowUpToDate] = useState(false);

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

  const handleClick = () => {
    if (isBlurred) {
      setIsBlurred(false);
      resetInactivityTimer();
    }
  };

  const handleThemeChange = (newTheme: Theme) => {
    setTheme(newTheme);
    // Save theme preference to local storage
    localStorage.setItem('theme', newTheme);
  };

  const handleImportComplete = async () => {
    // Refresh the entries list after import
    const entries = await invoke<Entry[]>('get_entries');
    setEntries(entries);
    // If there are entries, select the most recent one
    if (entries.length > 0) {
      setSelectedId(entries[0].id);
    }
  };

  const handleCreateNewEntry = async () => {
    try {
      const id = await createNewEntry();
      refreshEntries();
      setSelectedId(id);
      setShowSettings(false);
    } catch (err) {
      console.error('Failed to create new entry:', err);
    }
  };

  useEffect(() => {
    // Load saved theme preference
    const savedTheme = localStorage.getItem('theme') as Theme | null;
    if (savedTheme) {
      setTheme(savedTheme);
    }
  }, []);

  useEffect(() => {
    // Apply theme based on system preference or manual selection
    const applyTheme = () => {
      const isDarkMode = theme === 'dark' ||
        (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
      document.documentElement.setAttribute('data-theme', isDarkMode ? 'dark' : 'light');
    };

    applyTheme();

    // Listen for system theme changes
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = () => {
      if (theme === 'system') {
        applyTheme();
      }
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, [theme]);

  useEffect(() => {
    (async () => {
      const version = await getVersion();
      setAppVersion(version);
      const lastSeen = localStorage.getItem("lastSeenVersion");
      if (lastSeen && lastSeen !== version) {
        setShowChangelog(true);
      }
      localStorage.setItem("lastSeenVersion", version);
    })();
  }, []);

  useEffect(() => {
    invoke<Entry[]>("get_entries")
    .then((entries) => {
      setEntries(entries);
      if (entries.length > 0) {
        setSelectedId(entries[0].id);
      }
    })
    .catch((err) => console.error("Failed to fetch entries:", err));
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
      if (e.ctrlKey && e.key === 'n') {
        handleCreateNewEntry();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);


useEffect(() => {
  const unlisten = listen('open-settings', () => {
    setShowSettings(true);
  });

  return () => {
    unlisten.then((f) => f());
  };
}, []);

const updateRef = useRef<any>(null);
const [updateInfo, setUpdateInfo] = useState<{ version: string; body?: string | null } | null>(null);

const performUpdateCheck = async () => {
  const update = await check();
  if (!update) {
    return;
  }
  updateRef.current = update;
  setUpdateInfo({ version: update.version, body: update.body });
};

useEffect(() => {
  performUpdateCheck();
}, []);

useEffect(() => {
  const unlisten = listen('check-for-updates', async () => {
    const update = await check();
    if (!update) {
      setShowUpToDate(true);
      return;
    }
    updateRef.current = update;
    setUpdateInfo({ version: update.version, body: update.body });
  });

  return () => {
    unlisten.then((f) => f());
  };
}, []);

useEffect( () => {
  const unlistenNew = listen('new-entry', () => {
    handleCreateNewEntry();
  })

  const unlistenBlur = listen('blur', () => {
    setIsBlurred(prev => !prev);
  });

  return () => {
    unlistenNew.then(f => f());
    unlistenBlur.then(f => f());
  };
})

const handleInstallUpdate = async () => {
  if (!updateRef.current) return;
  setUpdateInfo(null);
  try {
    await updateRef.current.downloadAndInstall();
    await relaunch();
  } catch (err) {
    console.error('Update installation failed:', err);
  }
};

const handleDismissUpdate = () => setUpdateInfo(null);

const handleDismissUpToDate = () => setShowUpToDate(false);

  return (
    <>
      {updateInfo && (
        <Modal
          visible={true}
          header={`A newer version of Journal is available (${updateInfo.version})`}
          onClose={handleDismissUpdate}
          primaryButton={{ label: 'Install & Restart', onClick: handleInstallUpdate }}
          secondaryButton={{ label: 'Later', onClick: handleDismissUpdate }}
          body={
            <>
            <p className="whitespace-pre-line text-black">
              {updateInfo.body ?? 'A new version is available.'}
            </p>
            </>
          }
        />
      )}
      {showUpToDate && (
        <Modal
          visible={true}
          header="You're up to date!"
          onClose={handleDismissUpToDate}
          primaryButton={{ label: 'Cool!', onClick: handleDismissUpToDate }}
          body={
            <p className="whitespace-pre-line">
              The latest version is {appVersion} and you're on it 👍
            </p>
          }
        />
      )}
      <div 
        onClick={handleClick}
        style={{ 
          display: "flex",
          flexDirection: "column", 
          height: "100vh", 
          margin: 0, 
          padding: 0, 
          overflow: "hidden",
          filter: isBlurred ? "blur(8px)" : "none",
          transition: "filter 0.3s ease",
          cursor: isBlurred ? "pointer" : "default",
          backgroundColor: 'var(--background-color)',
          color: 'var(--text-color)'
        }}
      >
        <Modal
          visible={showChangelog}
          header={`What's new in v${appVersion}!`}
          body={
            <ul className="list-disc list-outside pl-5 space-y-2 marker:mr-2">
              {((changelog as Changelog)[appVersion] || []).map((item, idx) => (
                <li key={idx}>{item}</li>
              ))}
            </ul>
          }
          onClose={() => setShowChangelog(false)}
        />
        <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
          <div className="sidebar">
          <div className="sidebar-header">
            <NotebookPen onClick={handleCreateNewEntry}
              className="icon new-entry"
              role="button"
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") handleCreateNewEntry();
              }}
              style={{ cursor: "pointer" }}
              size={20}
            />
          </div>  
          <div className="entry-list-wrapper" style={{ flex: 1, overflowY: 'auto' }}>
            <EntryList 
                entries={entries}
                onSelect={(id) => {
                  setSelectedId(id);
                  setShowSettings(false);
                }} 
                activeId={selectedId}
                refreshEntries={refreshEntries}
                updateEntryTitle={updateEntryTitle}
            />
          </div>
            <div className="sidebar-footer">
              <Cog 
                onClick={() => setShowSettings(!showSettings)}
                className="icon settings" 
                size={20}
              />
            </div>
          </div>
          <div style={{ 
            flex: 1, 
            margin: 0, 
            padding: 0, 
            height: "100vh", 
            overflow: "hidden"
          }}>
            {showSettings ? (
              <div className="settings-container">
                <div className="settings-header">
                  <X
                    onClick={() => setShowSettings(false)}
                    size={24}
                  />
                </div>
                <Settings 
                  currentTheme={theme}
                  onThemeChange={handleThemeChange}
                  onImportComplete={handleImportComplete}
                />
              </div>
            ) : (
              <EntryEditor 
                selectedId={selectedId} 
                refreshEntries={refreshEntries}
                updateEntryTitle={updateEntryTitle}
              />
            )}
          </div>

        </div>
        
      </div>
    </>
  );
}
