# Portly

**Portly** is a command-line utility for discovering and assigning available ports on your local machine. It is designed to be fast, reliable, and easy to integrate into your development workflow.

---

## Features

- **Port Discovery**: Scan for available ports within a specified range.
- **Port Assignment**: Assign a port to your application and save it to an environment file.
- **Port Reuse**: Reuse previously assigned ports if they are still available.
- **Expandable Range**: Automatically expand the port range if no ports are found.
- **Process Ownership Check**: Verify if a port is owned by a specific application (using `lsof` or `pm2`).

---

## Installation

### Prerequisites

- Rust (1.60 or later)
- `lsof` (for port ownership checks)
- `pm2` (optional, for process ownership checks)

### Build from Source

1. Clone this repository:
   ```bash
   git clone https://github.com/oscarbrehier/portly.git
   cd portly
   ```
2. Build and install:
   ```bash
   cargo build --release
   cargo install --path .
   ```

### Download Prebuilt Binaries

You can download prebuilt binaries for your platform from the [GitHub Releases page](https://github.com/oscarbrehier/portly/releases).

**Linux (x86_64)**

```bash
curl -LO https://github.com/oscarbrehier/portly/releases/download/v0.1.0/portly-x86_64-linux
chmod +x portly-x86_64-linux
./portly-x86_64-linux

---

## Usage

### Basic Usage

```bash
portly
```

This will scan for an available port between 3000 and 8000, assign it to the environment variable `PORT`, and save it to `.portly.env`. The basic portly command supports all the same options as the port subcommand.

### Subcommand Usage

```bash
portly port --min 3000 --max 8000 --key PORT --app-name myapp
```

This will scan for a port, check if it is owned by `myapp`, and reuse it if possible.

### Options

| Option         | Description                                                     |
| -------------- | --------------------------------------------------------------- |
| `--min`        | Minimum port number to scan (default: 3000)                     |
| `--max`        | Maximum port number to scan (default: 8000)                     |
| `--key`        | Environment variable key (default: `PORT`)                      |
| `--app-name`   | Name of the application to check for port ownership             |
| `--forced`     | Force a new port assignment, ignoring previously assigned ports |
| `--env-file`   | File to store the assigned port (default: `.portly.env`)        |
| `--expand-max` | Expand the maximum port range if no ports are found             |

---

## Examples

### Assign a Port

```bash
portly --min 3000 --max 8000 --key MY_APP_PORT
```

This will assign an available port to `MY_APP_PORT` and save it to `.portly.env`.

### Reuse a Port

```bash
portly --min 3000 --max 8000 --key MY_APP_PORT --app-name myapp
```

This will reuse the previously assigned port for `myapp` if it is still available.

---

## Legacy JavaScript Version

The original JavaScript version of Portly is archived in the [`legacy/js/`](legacy/js/) directory. It is no longer maintained, but you can refer to it if needed.

---

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

---

## License

This project is licensed under the MIT License.
