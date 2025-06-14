# represence

*Like Discord Rich Presence, but for anywhere!*

A lightweight, adaptive presence detection system that monitors your current activity and provides real-time status updates via a REST API and WebSocket connection.

## ✨ Features

- **Smart Application Detection**: Automatically detects and prioritizes running applications
- **VSCode Integration**: Deep integration with VS Code to show current file and project
- **Tiered Priority System**: Prioritizes coding activities over browsing or entertainment
- **Real-time Updates**: WebSocket support for live presence streaming
- **Adaptive Polling**: Faster updates when active, slower when idle for optimal performance
- **REST API**: Simple HTTP endpoints for presence data
- **Lightweight**: Optimized Rust backend with minimal resource usage

## 🎯 Detected Applications

The system intelligently detects and categorizes applications by priority:

**Tier 1 (Highest Priority)**
- `code` - Visual Studio Code (shows current file when extension is installed)
- `discord` - Discord

**Tier 2 (Work & Browsing)**
- `zen` - Zen Browser
- `chrome` - Google Chrome  
- `steam` - Steam

**Tier 3 (Entertainment)**
- `vlc` - VLC Media Player
- `stremio` - Stremio

**Tier 4 (Development Tools)**
- `ghostty` - Ghostty Terminal

## 🚀 Quick Start

### Prerequisites
- [Rust](https://rustup.rs/) (latest stable version)
- Git

### Installation

1. **Clone and build:**
   ```bash
   git clone https://github.com/bilgi42/represence
   cd represence
   cargo build --release
   ```

2. **Install globally:**
   ```bash
   # Install via cargo (recommended)
   cargo install --path .
   
   # Or copy manually
   sudo cp target/release/represence /usr/local/bin/
   sudo chmod +x /usr/local/bin/represence
   ```

3. **Run the service:**
   ```bash
   # Run directly
   represence
   
   # Or run in background
   nohup represence > /dev/null 2>&1 &
   ```

### VSCode Extension (Optional but Recommended)

For detailed file information when coding, install the companion VSCode extension:

1. Download `represence-vscode-0.0.2.vsix` from the `represence-vscode` directory
2. Install: `code --install-extension represence-vscode-0.0.2.vsix`
3. The extension automatically starts when VSCode launches

## ⚙️ Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `REPRESENCE_VSCODE_PORT` | `3847` | Port for VSCode extension WebSocket connection |

### Example Configuration
```bash
export REPRESENCE_VSCODE_PORT=3847
represence
```

## 🌐 API Reference

The service runs on `http://localhost:3001` with the following endpoints:

### REST Endpoints

#### `GET /api/represence`
Get current presence data.

**Response:**
```json
{
  "text": "editing main.rs in Visual Studio Code"
}
```

#### `GET /health`
Health check and service information.

**Response:**
```json
{
  "status": "healthy",
  "timestamp": 1704067200,
  "version": "0.1.0",
  "endpoints": {
    "presence": "/api/represence",
    "websocket": "/ws/represence", 
    "health": "/health"
  }
}
```

#### `GET /`
API information and welcome message.

### WebSocket Endpoint

#### `WS /ws/represence`
Real-time presence updates via WebSocket.

**Example Usage:**
```javascript
const ws = new WebSocket('ws://localhost:3001/ws/represence');
ws.onmessage = (event) => {
  const presence = JSON.parse(event.data);
  console.log('Current activity:', presence.text);
};
```

## 🔧 Running as a Service

### systemd (Linux)

1. **Create service file:**
   ```bash
   sudo tee /etc/systemd/system/represence.service << EOF
   [Unit]
   Description=Represence Rich Presence Service
   After=network.target

   [Service]
   Type=simple
   User=$USER
   ExecStart=$(which represence)
   Restart=always
   RestartSec=3
   StandardOutput=journal
   StandardError=journal

   [Install]
   WantedBy=multi-user.target
   EOF
   ```

2. **Enable and start:**
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable represence
   sudo systemctl start represence
   sudo systemctl status represence
   ```

## 💡 Usage Examples

### Fetch Current Status
```bash
curl http://localhost:3001/api/represence
# Output: {"text":"editing README.md in Visual Studio Code"}
```

### Monitor Activity
```bash
#!/bin/bash
while true; do
  STATUS=$(curl -s http://localhost:3001/api/represence | jq -r '.text')
  echo "$(date '+%H:%M:%S'): $STATUS"
  sleep 10
done
```

### JavaScript Integration
```javascript
async function getCurrentActivity() {
  const response = await fetch('http://localhost:3001/api/represence');
  const data = await response.json();
  return data.text;
}

// Usage
getCurrentActivity().then(activity => {
  document.getElementById('status').textContent = activity;
});
```

## 🔍 How It Works

1. **Process Detection**: Scans `/proc` directory for running applications
2. **Smart Caching**: Caches process information with change detection
3. **Adaptive Timing**: 1-second updates when active, 3-second when idle
4. **VSCode Integration**: Connects to VSCode extension via WebSocket for file details
5. **Priority System**: Shows highest-priority activity from detected applications

## 🛠️ Development

### Building from Source
```bash
git clone https://github.com/bilgi42/represence
cd represence
cargo build --release
```

### Running Tests
```bash
cargo test
```

### Contributing
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## 🗑️ Uninstalling

```bash
# Stop service (if using systemd)
sudo systemctl stop represence
sudo systemctl disable represence
sudo rm /etc/systemd/system/represence.service
sudo systemctl daemon-reload

# Remove binary
cargo uninstall represence
# Or if installed manually:
sudo rm /usr/local/bin/represence
```

## 📝 License

This project is open source. See the LICENSE file for details.

---

*Made with ❤️ for developers who want to share their current vibe*

