# represence
like discord rich presence, but for anywhere!

Stream your rich presence data, and fetch it to use however you'd like

## Detailed data
Detailed data stream is only available for VSCode, with an help of an extension. I am open to PR's for more functionality

## Installation

### Prerequisites
- [Rust](https://rustup.rs/) (latest stable version)
- Git

### Build and Install

1. **Clone the repository:**
   ```bash
   git clone https://github.com/bilgi42/represence
   cd represence
   ```

2. **Build the project:**
   ```bash
   cargo build --release
   ```

3. **Install globally:**
   ```bash
   # Install to your system PATH
   cargo install --path .
   ```

   Or manually copy the binary:
   ```bash
   # For manual installation, copy to a directory in your PATH
   sudo cp target/release/represence /usr/local/bin/
   # Make it executable
   sudo chmod +x /usr/local/bin/represence
   ```

### Running as a Background Service

#### Option 1: Using systemd (recommended for Linux)

1. **Create a systemd service file:**
   ```bash
   sudo nano /etc/systemd/system/represence.service
   ```

2. **Add the following content:**
   ```ini
   [Unit]
   Description=Represence Rich Presence Service
   After=network.target

   [Service]
   Type=simple
   User=YOUR_USERNAME
   ExecStart=/usr/local/bin/represence
   Restart=always
   RestartSec=3
   StandardOutput=null
   StandardError=null

   [Install]
   WantedBy=multi-user.target
   ```
   
   Replace `YOUR_USERNAME` with your actual username.

3. **Enable and start the service:**
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable represence
   sudo systemctl start represence
   ```

4. **Check service status:**
   ```bash
   sudo systemctl status represence
   ```

#### Option 2: Running in background manually

```bash
# Run in background with nohup
nohup represence > /dev/null 2>&1 &

# Or run detached
represence &
disown
```

### Usage

Once installed, you can:

- **Start the service:** `represence`
- **Run in background:** `represence &`
- **Check if running:** `ps aux | grep represence`

### Configuration

The service will run silently in the background. By default, it should start automatically after installation as a systemd service.

#### Environment Variables

You can configure the service using environment variables:

```bash
# Allow specific domains to access the API (comma-separated)
export REPRESENCE_DOMAIN_ALLOWED="https://your-vm-ip:3000,https://localhost:3000"

# Configure VS Code extension port (default: 3847)
export REPRESENCE_VSCODE_PORT=3847
```

#### Remote Access Configuration

To send your presence data to your Ubuntu server VM:

1. **Find your machine's IP address:**
   ```bash
   ip addr show | grep inet
   # Or for external IP:
   curl ifconfig.me
   ```

2. **Configure firewall (if needed):**
   ```bash
   # Allow port 3001 through firewall
   sudo ufw allow 3001
   # Or for specific IP only:
   sudo ufw allow from YOUR_VM_IP to any port 3001
   ```

3. **Set environment variables for remote access:**
   ```bash
   # Create environment file
   sudo mkdir -p /etc/represence
   sudo tee /etc/represence/config.env << EOF
   REPRESENCE_DOMAIN_ALLOWED="https://YOUR_VM_IP:3000,http://YOUR_VM_IP:3000"
   REPRESENCE_VSCODE_PORT=3847
   EOF
   ```

4. **Update systemd service to use environment file:**
   ```bash
   sudo nano /etc/systemd/system/represence.service
   ```
   
   Update the service file to include the environment file:
   ```ini
   [Unit]
   Description=Represence Rich Presence Service
   After=network.target

   [Service]
   Type=simple
   User=YOUR_USERNAME
   EnvironmentFile=/etc/represence/config.env
   ExecStart=/usr/local/bin/represence
   Restart=always
   RestartSec=3
   StandardOutput=null
   StandardError=null

   [Install]
   WantedBy=multi-user.target
   ```

5. **Restart the service:**
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl restart represence
   ```

#### Accessing from Your Ubuntu VM

From your Ubuntu server VM, you can fetch the presence data:

```bash
# Replace YOUR_MACHINE_IP with your actual machine's IP
curl http://YOUR_MACHINE_IP:3001/api/presence

# Example response:
# {"text":"editing main.rs in Visual Studio Code"}

# Health check endpoint:
curl http://YOUR_MACHINE_IP:3001/health
```

#### API Endpoints

- **GET** `/api/presence` - Get current presence data
- **GET** `/health` - Health check and configuration info  
- **GET** `/` - API information

#### Example Integration Script for Ubuntu VM

Create a script on your Ubuntu VM to fetch and use the presence data:

```bash
#!/bin/bash
# save as fetch_presence.sh on your Ubuntu VM

REPRESENCE_HOST="YOUR_MACHINE_IP:3001"

while true; do
    PRESENCE=$(curl -s "http://$REPRESENCE_HOST/api/presence" | jq -r '.text')
    echo "$(date): $PRESENCE"
    
    # Do something with the presence data
    # For example, log it or send to another service
    
    sleep 30
done
```

### Uninstalling

```bash
# Stop the service
sudo systemctl stop represence
sudo systemctl disable represence

# Remove the service file
sudo rm /etc/systemd/system/represence.service
sudo systemctl daemon-reload

# Remove the binary
sudo rm /usr/local/bin/represence

# Or if installed via cargo
cargo uninstall represence
```

