import { useState, useEffect, useRef } from "react";
import { getVersion } from "@tauri-apps/api/app";
import EntryEditor from "./components/EntryEditor";
import EntryList from "./components/EntryList";
import Settings from "./components/Settings";
import Modal from "./components/Modal";
import { invoke } from "@tauri-apps/api/core";
import { NotebookPen, Cog, ChevronLeft, ChevronRight } from 'lucide-react';
import { createNewEntry } from './lib/createEntry';
import { X } from 'lucide-react';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { listen } from '@tauri-apps/api/event';
import changelog from "../changelog.json";


type Changelog = Record<string, string[]>;

type Entry = {
  id: number;
  title: string;
  created_at: string;
};

type Theme = 'system' | 'light' | 'dark';

type KeychainStatus = "unknown" | "authorized" | "error" | "checking";

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
  const [isCollapsed, setIsCollapsed] = useState(false);

  const [showUpToDate, setShowUpToDate] = useState(false);

  // Session‑scoped flag: we remember the authorization only for the lifetime
  // of the current window. Fresh app launches will start as "unknown" again.
  const initialSessionAuthorized =
    sessionStorage.getItem("sessionAuthorized") === "true";
  const [keychainStatus, setKeychainStatus] = useState<KeychainStatus>(
    initialSessionAuthorized ? "authorized" : "unknown"
  );
  const [keychainError, setKeychainError] = useState<string | null>(null);

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



  const handleAuthorizeKeychain = async () => {
    setKeychainError(null);
    try {
      await invoke("authorize_keychain_command");
      setKeychainStatus("authorized");
      // Remember for the rest of this session (window). Not persisted across re‑launches.
      sessionStorage.setItem("sessionAuthorized", "true");
    } catch (err: any) {
      setKeychainStatus("error");
      setKeychainError(err?.toString() || "Failed to access keychain.");
    }
  };

  // Load entries once we are authorized
  useEffect(() => {
    if (keychainStatus === "authorized") {
      (async () => {
        const loadedEntries = await invoke<Entry[]>("get_entries");
        if (loadedEntries.length === 0) {
          // No entries: create a new one, then reload
          const newId = await createNewEntry();
          const newEntries = await invoke<Entry[]>("get_entries");
          setEntries(newEntries);
          setSelectedId(newId);
        } else {
          setEntries(loadedEntries);
          if (selectedId === null) {
            setSelectedId(loadedEntries[0].id);
          }
        }
      })();
    }
  }, [keychainStatus]);


  return (
    <>
      {/* Keychain authorization modal - only show if not authorized */}
      {keychainStatus !== "authorized" && (
        <Modal
          visible={true}
          header={keychainStatus === "error" ? "Keychain access error" : "Keychain access required"}
          body={
            <div>
              {keychainStatus === "error" ? (
                <>
                  <p className="text-red-600 mb-2">{keychainError || "Failed to access the system keychain."}</p>
                  <p>Please grant access to the keychain to view and create encrypted journal entries.</p>
                </>
              ) : (
                <>
                  <p>This app uses the macOS Keychain to securely encrypt your journal. We need your permission to access the keychain </p>
                  <p className="mt-2">If you click 'Always Allow' you only have to do this once, unless you reset your keychain or reinstall the app.</p>
                </>
              )}
            </div>
          }
          onClose={() => {}}
          primaryButton={{
            label: keychainStatus === "error" ? "Try Again" : "Authorize",
            onClick: handleAuthorizeKeychain,
          }}
        />
      )}
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
      {/* Only render the main app if authorized */}
      {keychainStatus === "authorized" && (
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
            <div className="sidebar" style={{
              width: isCollapsed ? "50px" : "250px",
              transition: "width 0.3s ease",
              overflow: "hidden",
              display: "flex",
              flexDirection: "column"
            }}>
            <div className="sidebar-header" style={{ opacity: isCollapsed ? 0 : 1, transition: "opacity 0.2s ease" }}>
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
            <div className="entry-list-wrapper" style={{ flex: 1, overflowY: isCollapsed ? 'hidden' : 'auto', opacity: isCollapsed ? 0 : 1, transition: "opacity 0.2s ease" }}>
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
              <div className="sidebar-footer" style={{ opacity: isCollapsed ? 0 : 1, transition: "opacity 0.2s ease" }}>
                <Cog 
                  onClick={() => setShowSettings(!showSettings)}
                  className="icon settings" 
                  size={20}
                />
              </div>
            </div>
            <button
              onClick={() => setIsCollapsed(!isCollapsed)}
              style={{
                position: "absolute",
                left: isCollapsed ? "42px" : "242px", // Adjust left position based on collapsed state
                top: "50%", // Vertically center
                transform: "translateY(-50%)", // Adjust for button height
                background: "var(--background-color)",
                border: "1px solid var(--border-color)",
                borderRadius: "50%",
                cursor: "pointer",
                padding: "0.25rem",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "var(--text-color)",
                zIndex: 10, // Ensure button is above other content
                transition: "left 0.3s ease, background-color 0.3s ease, border-color 0.3s ease"
              }}
            >
              {isCollapsed ? <ChevronRight size={16} /> : <ChevronLeft size={16} />}
            </button>
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
                entries.length === 0 ? (
                  <div style={{
                    display: 'flex',
                    flexDirection: 'column',
                    alignItems: 'center',
                    justifyContent: 'center',
                    height: '100%',
                  }}>
                    <h2 className="editor-title font-serif" style={{ marginBottom: '1rem', textAlign: 'center' }}>Nothing here yet</h2>
                    <button
                      style={{
                        padding: '0.75rem 1.5rem',
                        fontSize: '1rem',
                        borderRadius: '0.5rem',
                        border: 'none',
                        background: '#0070f3',
                        color: 'white',
                        cursor: 'pointer',
                        boxShadow: '0 2px 8px rgba(0,0,0,0.04)'
                      }}
                      onClick={handleCreateNewEntry}
                    >
                      Add your first entry
                    </button>
                  </div>
                ) : (
                  <EntryEditor
                    key={selectedId}
                    selectedId={selectedId}
                    refreshEntries={refreshEntries}
                    updateEntryTitle={updateEntryTitle}
                  />
                )
              )}
            </div>

          </div>
          
        </div>
      )}
    </>
  );
}
