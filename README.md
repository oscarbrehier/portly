# Portly

Portly is a Node.js utility to automate dynamic port assignment, generate Nginx configurations, and prepare your app for deployment. It’s designed to integrate easily into CI/CD pipelines and works well with Express, PM2, or any Node.js web app.

---

## Features

- Finds an **available port** within a configurable range.  
- Generates **Nginx config files** from a template (`nginx_template.txt`).  
- Exports the selected port as an **environment variable** (you choose the name).  
- Outputs the config file to a specified directory.  
- Works seamlessly with **CI/CD pipelines** like CircleCI, GitHub Actions, etc.

---

## Installation

```bash
git clone https://github.com/oscarbrehier/
cd portly
npm install
```

## Usage
1. Create an Nginx template file named nginx_template.txt in the project root:

```nginx
server {
    listen {{PORT}};
    server_name {{DOMAIN}};

    location / {
        proxy_pass http://localhost:{{PORT}};
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }
}
```

2. Run Portly on your server:
```bash
PORT_ENV_NAME=<YOUR_ENV_VAR_NAME> node index.js
```
🧠 Replace `<YOUR_ENV_VAR_NAME>` with whatever you want to name the environment variable
that will hold the port number — for example:

Output:
```bash
Available port found: 3001
export <YOUR_ENV_VAR_NAME>=3001
Config written to /home/deployer/portly/nginx-configs/example.com
```

You can now use that variable in your deployment:
```bash
PORT=$<YOUR_ENV_VAR_NAME> pm2 start ./build/index.js --name my-app --update-env
```

## Environment Variables
| Variable | Description | Default |
|-----------|--------------|----------|
| `PORT_MIN` | Minimum port number to scan | `3000` |
| `PORT_MAX` | Maximum port number to scan | `PORT_MIN + 600` |
| `PORT_ENV_NAME` | The name of the environment variable that stores the found port | **Required** |
| `DOMAIN` | Your app’s domain name (used in Nginx config) | **Required** |



MIT License
Copyright ©
