# Journal
<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Journal Logo" width="128" />
</p

A lightweight journaling app for macOS, built with privacy as its core principle. No cloud. No syncing. No sharing. Just your thoughts — stored locally, and seen by no one but you. A calm, focused writing experience made just for your Mac.

### Features
<p align="center">
  <img src="screenshot.png" alt="Journal Screenshot"/>
</p

* **Backups the old fashioned way**: Your journal data is stored in `~/Library/Application Support/Journal/journal.db`. You can export this to your Downloads folder in Settings to save to an external hard drive or import old journal entries to view in your Journal.
* **Ctrl + b**: Blur your screen when it's not in use. Also blurs automatically after a minute of non-use. Click to dismiss or hit Crtl+b again.
* **Dark mode**: Toggleable in Settings between System, Light, and Dark.
* **Encryption at rest**: AES-256 encryption on your local journal entries database.