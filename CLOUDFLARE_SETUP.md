# Cloudflare Tunnel Setup for Represence

This guide will help you expose your Represence API to the internet using Cloudflare Tunnel, allowing your website to fetch your real-time presence data from anywhere.

## Prerequisites

- A domain managed by Cloudflare (free tier is fine)
- Cloudflare account
- `cloudflared` installed on your Ubuntu VM
- Your Represence service running on port 3001
- VS Code extension using port 3847 (changed from 3000 to avoid conflicts)

## Step 1: Install Cloudflared (if not already installed)

```bash
# Download and install cloudflared
curl -L --output cloudflared.deb \
  https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb

sudo dpkg -i cloudflared.deb

# Verify installation
cloudflared --version
```

## Step 2: Authenticate with Cloudflare

```bash
cloudflared tunnel login
```

This will:
- Open a browser window
- Ask you to select your domain
- Download a certificate to `~/.cloudflared/cert.pem`

## Step 3: Create a Named Tunnel

```bash
cloudflared tunnel create represence
```

This creates:
- A tunnel with UUID (save this!)
- Credentials file at `~/.cloudflared/[UUID].json`

**Example output:**
```
Created tunnel represence with id 12345678-1234-1234-1234-123456789012
```

## Step 4: Configure DNS Routing

Replace `your-domain.com` with your actual domain:

```bash
cloudflared tunnel route dns represence presence.your-domain.com
```

This creates a CNAME record pointing `presence.your-domain.com` to your tunnel.

## Step 5: Create Configuration File

Create `~/.cloudflared/config.yml`:

```yaml
tunnel: represence
credentials-file: /home/YOUR_USERNAME/.cloudflared/12345678-1234-1234-1234-123456789012.json

ingress:
  - hostname: presence.your-domain.com
    service: http://localhost:3001
  # Catch-all rule (required)
  - service: http_status:404
```

**Replace:**
- `YOUR_USERNAME` with your actual username for the linux VM
- `12345678-1234-1234-1234-123456789012` with your tunnel UUID
- `your-domain.com` with your actual domain

## Step 6: Configure Your .env File

Update your `.env` file in the Represence project:

```bash
# Edit your .env file
nano .env
```

Set your domain(s) - you can specify multiple domains separated by commas:
```
REPRESENCE_DOMAIN_ALLOWED=https://your-website.com
```

**Examples:**
```
# For a single domain
REPRESENCE_DOMAIN_ALLOWED=https://bilgilovelace.com

# For multiple domains (comma-separated)
REPRESENCE_DOMAIN_ALLOWED=https://bilgilovelace.com,https://www.bilgilovelace.com

# For multiple environments
REPRESENCE_DOMAIN_ALLOWED=https://bilgilovelace.com,https://staging.bilgilovelace.com,http://localhost:3000

# For GitHub Pages + custom domain
REPRESENCE_DOMAIN_ALLOWED=https://bilgilovelace.github.io,https://bilgilovelace.com

# For local testing only
REPRESENCE_DOMAIN_ALLOWED=http://localhost:3000
```

## Step 7: Start Your Services

### Terminal 1: Start Represence
```bash
cd /path/to/represence
cargo run
```

### Terminal 2: Start Cloudflare Tunnel
```bash
cloudflared tunnel --config ~/.cloudflared/config.yml run represence
```

## Step 8: Test Your Setup

### Test the tunnel:
```bash
curl https://presence.your-domain.com/health
```

**Expected response:**
```json
{
  "status": "healthy",
  "timestamp": 1672531200,
  "allowed_domains": ["https://your-website.com"]
}
```

### Test the presence endpoint:
```bash
curl https://presence.your-domain.com/api/presence
```

**Expected response:**
```json
{
  "text": "editing main.rs"
}
```

## Step 9: Make Tunnel Persistent (Optional)

To start the tunnel automatically on boot:

```bash
sudo cloudflared service install
```

To start/stop the service:
```bash
sudo systemctl start cloudflared
sudo systemctl stop cloudflared
sudo systemctl status cloudflared
```

## Step 10: Using in Your Website

### JavaScript Example:
```javascript
// Fetch presence data
async function updatePresence() {
    try {
        const response = await fetch('https://presence.your-domain.com/api/presence');
        const data = await response.json();
        
        // Update your website
        document.getElementById('status').textContent = data.text;
        console.log('Current activity:', data.text);
    } catch (error) {
        console.error('Failed to fetch presence:', error);
        document.getElementById('status').textContent = 'Offline';
    }
}

// Update every 10 seconds
updatePresence();
setInterval(updatePresence, 10000);
```

### HTML Example:
```html
<!DOCTYPE html>
<html>
<head>
    <title>My Website</title>
</head>
<body>
    <div class="presence-widget">
        <h3>What I'm doing right now:</h3>
        <p id="status">Loading...</p>
        <small id="last-updated"></small>
    </div>

    <script>
        async function updatePresence() {
            try {
                const response = await fetch('https://presence.your-domain.com/api/presence');
                const data = await response.json();
                
                document.getElementById('status').textContent = data.text;
                document.getElementById('last-updated').textContent = 
                    'Last updated: ' + new Date().toLocaleTimeString();
            } catch (error) {
                document.getElementById('status').textContent = 'Offline';
            }
        }
        
        updatePresence();
        setInterval(updatePresence, 10000);
    </script>
</body>
</html>
```

## Troubleshooting

### Common Issues:

1. **CORS Error**: Make sure your `.env` file has the correct domain(s) - use comma-separated list for multiple domains
2. **404 Error**: Check that your tunnel configuration is correct
3. **Connection Refused**: Ensure Represence is running on port 3001
4. **DNS Not Resolving**: Wait a few minutes for DNS propagation

### Debug Commands:
```bash
# Check tunnel status
cloudflared tunnel info represence

# Check DNS records
dig presence.your-domain.com

# Test local service
curl http://localhost:3001/health

# Check tunnel logs
cloudflared tunnel --config ~/.cloudflared/config.yml run represence --loglevel debug
```

### Log Locations:
- Tunnel logs: `/var/log/cloudflared.log`
- Service logs: `journalctl -u cloudflared`

## Security Notes

- Your API only accepts requests from the domain specified in `REPRESENCE_DOMAIN_ALLOWED`
- Cloudflare provides DDoS protection and SSL automatically
- The tunnel creates an encrypted connection to Cloudflare's edge
- No ports need to be opened on your firewall

## API Endpoints

Once set up, you'll have these endpoints available:

- `GET https://presence.your-domain.com/` - API information
- `GET https://presence.your-domain.com/api/presence` - Current presence data
- `GET https://presence.your-domain.com/health` - Health check with configuration info

## Example Responses

### Presence Endpoint:
```json
{
  "text": "editing main.rs"
}
```

### Health Endpoint:
```json
{
  "status": "healthy",
  "timestamp": 1672531200,
  "allowed_domains": ["https://your-website.com", "https://www.your-website.com"]
}
```

## Next Steps

1. Add the presence widget to your website
2. Style it to match your site's design
3. Consider adding error handling and fallback states
4. Monitor the logs to ensure everything is working smoothly

Your presence data is now available globally! 🎉 