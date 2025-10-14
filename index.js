import portfinder from "portfinder";
import fs from "fs";
import path from "path";
import net from "net";

const PORT_MIN = process.env.PORT_MIN ? Number(process.env.PORT_MIN) : 3000;
const PORT_MAX = process.env.PORT_MAX ? Number(process.env.PORT_MAX) : PORT_MIN + 600;

class Portly {
	constructor(portEnvName, templatePath, domain, placeholders) {

		this.portEnvName = portEnvName;
		this.templatePath = templatePath;
		this.domain = domain;
		this.placeholders = placeholders;
		this.content = null;
		this.__dirname = path.dirname(new URL(import.meta.url).pathname);
		this.server = null;

	};

	async getConfiguration() {
		
		let availablePort;

		portfinder.setBasePort(PORT_MIN);
		portfinder.setHighestPort(PORT_MAX);

		try {
			availablePort = await portfinder.getPortPromise();
		} catch (err) {
			throw new Error(`Error finding available port: ${err}`);
		};

		if (!availablePort) return this;

		console.log(`Found available port: ${availablePort}`);

		await this.holdPort(availablePort);

		process.env[this.portEnvName] = availablePort;

		try {

			const templateFullPath = path.join(this.__dirname, this.templatePath);
			const data = await fs.promises.readFile(templateFullPath, "utf-8");

			let content = data;

			this.placeholders.forEach(([key, value]) => {
				content = content.replaceAll(`{{${key}}}`, key === "PORT" ? availablePort : value);
			});

			this.content = content;
			return this;

		} catch (err) {
			throw new Error(`Error reading template file: ${err}`);
		};

	};

	async holdPort(port) {

		return new Promise((resolve, reject) => {

			this.server = net.createServer();
			
			this.server.listen(port, '127.0.0.1', () => {
				console.log(`Port ${port} is now reserved`);
				resolve();
			});

			this.server.on('error', (err) => {
				reject(new Error(`Failed to hold port ${port}: ${err.message}`));
			});

		});

	};

	async writeConfigFile() {

		if (!this.content) return null;

		const configDir = path.join(this.__dirname, "./nginx-configs");
		const nginxConfigFilePath = path.join(configDir, this.domain);

		try {

			await fs.promises.mkdir(configDir, { recursive: true });
			await fs.promises.writeFile(nginxConfigFilePath, this.content, { encoding: "utf-8" });
			console.log(`Nginx config written`);

		} catch (err) {
			throw new Error(`Cannot write Nginx config: ${err}`);
		};

	};

	async writeEnvFile() {

		const envFilePath = path.join(this.__dirname, ".portly.env");
		const envContent = `export ${this.portEnvName}=${process.env[this.portEnvName]}\n`;

		try {

			await fs.promises.writeFile(envFilePath, envContent, { encoding: "utf-8" });
			console.log(`Environment file written`);

		} catch (err) {
			throw new Error(`Cannot write environment file: ${err}`);
		};

	};

	async writeReadyMarker() {

		const readyFile = path.join(this.__dirname, ".portly.ready");
		
		try {

			await fs.promises.writeFile(readyFile, Date.now().toString(), { encoding: "utf-8" });
			console.log(`Ready marker created`);

		} catch (err) {
			throw new Error(`Cannot write ready marker: ${err}`);
		};

	};

	keepAlive() {

		console.log(`\nPortly ready (PID: ${process.pid})`);
		console.log(`Holding port until shutdown signal\n`);

		const shutdown = () => {

			console.log("Releasing port");

			if (this.server) {
				this.server.close(() => {
					process.exit(0);
				});
			} else {
				process.exit(0);
			};

		};

		process.on("SIGTERM", shutdown);
		process.on("SIGINT", shutdown);

		setInterval(() => {}, 1000);
	};

};

(async () => {

	const domain = process.env.DOMAIN;
	const envName = process.env.PORT_ENV_NAME || "PORT";

	if (!domain) {
		console.error(`Missing DOMAIN environment variable`);
		process.exit(1);
	};

	try {

		const portly = new Portly(
			envName,
			"./nginx-template.txt",
			domain,
			[
				["DOMAIN", domain],
				["PORT", ""]
			]
		);

		await portly.getConfiguration();
		await portly.writeConfigFile();
		await portly.writeEnvFile();
		await portly.writeReadyMarker();

		portly.keepAlive();

	} catch (err) {
		console.error(`Portly failed: ${err}`);
		process.exit(1);
	};

})();