# Portly

**Portly** is a command-line utility for discovering and assigning available ports on your local machine. It is designed to be fast, reliable, and easy to integrate into your development workflow.

---

## Features
- **Port Discovery**: Scan for available ports within a specified range.
- **Port Assignment**: Assign a port to your application and save it to an environment file.
- **Port Reuse**: Reuse previously assigned ports if they are still available.
- **Expandable Range**: Automatically expand the port range if no ports are found.
- **Process Ownership Check**: Verify if a port is owned by a specific application (using `pm2` or `lsof`).

---

## Installation

### Prerequisites
- Rust (1.60 or later)
- `lsof` (for port ownership checks)
- `pm2` (optional, for process ownership checks)

### Build from Source
1. Clone this repository:
   ```bash
   git clone https://github.com/yourusername/portly.git
   cd portly
```