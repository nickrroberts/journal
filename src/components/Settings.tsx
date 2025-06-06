import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { confirm } from '@tauri-apps/plugin-dialog';
import { Select } from './Select';
import { getVersion } from '@tauri-apps/api/app';

type Theme = 'system' | 'light' | 'dark';

type Props = {
  currentTheme: Theme;
  onThemeChange: (theme: Theme) => void;
  onImportComplete: () => void;
};

export default function Settings({ currentTheme, onThemeChange, onImportComplete }: Props) {
  const [exportStatus, setExportStatus] = useState<string>('');
  const [importStatus, setImportStatus] = useState<string>('');
  const [deleteStatus, setDeleteStatus] = useState<string>('');
  const [appVersion, setAppVersion] = useState<string | null>(null);

  const handleExport = async () => {
    try {
      const path = await invoke<string>('export_database');
      setExportStatus(`Database exported to: ${path}`);
      setTimeout(() => setExportStatus(''), 3000);
    } catch (error) {
      setExportStatus(`Export failed: ${error}`);
      setTimeout(() => setExportStatus(''), 3000);
    }
  };

  const handleImport = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'SQLite Database',
          extensions: ['db']
        }]
      });

      if (selected && typeof selected === 'string') {
        await invoke('import_database', { path: selected });
        setImportStatus('Database imported successfully');
        onImportComplete();
        setTimeout(() => setImportStatus(''), 3000);
      }
    } catch (error) {
      setImportStatus(`Import failed: ${error}`);
      setTimeout(() => setImportStatus(''), 3000);
    }
  };

  const handleDeleteAll = async () => {
    const confirmed = await confirm(
      'Are you sure you want to delete all entries? This action cannot be undone.',
        {
          title: 'Delete all entries?',
          okLabel: 'Yes, delete',
          cancelLabel: 'Cancel'
        }
    );

    if (confirmed) {
      try {
        await invoke('delete_all_entries');
        setDeleteStatus('All entries deleted successfully');
        onImportComplete(); // This will refresh the entries list
        setTimeout(() => setDeleteStatus(''), 3000);
      } catch (error) {
        setDeleteStatus(`Failed to delete entries: ${error}`);
        setTimeout(() => setDeleteStatus(''), 3000);
      }
    }
  };

  
useEffect(() => {
  getVersion().then(setAppVersion);
}, []);

  return (
    <div style={{ padding: '1rem' }}>
      <h2 className='text-2xl font-semibold'>Settings</h2>
      
      <div style={{ marginBottom: '2rem' }}>
        <label style={{ display: 'block', marginBottom: '0.5rem' }} className='font-semibold'>Theme</label>
        <Select
        className="max-w-sm mx-auto my-4"
          value={currentTheme}
          onChange={(e) => onThemeChange(e.target.value as Theme)}
          options={[
            { label: 'System', value: 'system' },
            { label: 'Light', value: 'light' },
            { label: 'Dark', value: 'dark'}
          ]}
        />
      </div>

      <div style={{ marginBottom: '2rem' }}>
        <h3 style={{ marginBottom: '1rem' }} className='font-semibold'>Manage entries</h3>
        
        <div style={{ display: 'flex', gap: '1rem', marginBottom: '1rem' }}>
          <button
            onClick={handleExport}
            style={{
              padding: '0.5rem 1rem',
              borderRadius: '4px',
              border: '1px solid var(--text-color)',
              backgroundColor: 'var(--background-color)',
              color: 'var(--text-color)',
              cursor: 'pointer'
            }}
          >
            Export entries
          </button>
          
          <button
            onClick={handleImport}
            style={{
              padding: '0.5rem 1rem',
              borderRadius: '4px',
              border: '1px solid var(--text-color)',
              backgroundColor: 'var(--background-color)',
              color: 'var(--text-color)',
              cursor: 'pointer'
            }}
          >
            Import entries
          </button>

          <button
            onClick={handleDeleteAll}
            style={{
              padding: '0.5rem 1rem',
              borderRadius: '4px',
              border: '1px solid #ff4444',
              backgroundColor: 'var(--background-color)',
              color: '#ff4444',
              cursor: 'pointer'
            }}
          >
            Delete all entries
          </button>
        </div>

        {exportStatus && (
          <div style={{ color: 'var(--text-color)' }}>
            {exportStatus}
          </div>
        )}

        {importStatus && (
          <div style={{ color: 'var(--text-color)' }}>
            {importStatus}
          </div>
        )}

        {deleteStatus && (
          <div style={{ color: 'var(--text-color)' }}>
            {deleteStatus}
          </div>
        )}
      </div>
      <div style={{ marginTop: '2rem', fontSize: '0.9rem', color: 'var(--text-color)' }}>
        Version: {appVersion}
      </div>
    </div>
  );
} 