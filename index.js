import "dotenv/config";
import portfinder from "portfinder";
import fs from "fs";
import path from "path";
import net from "net";
import { exec } from "child_process";

const PORT_MIN = process.env.PORT_MIN ? Number(process.env.PORT_MIN) : 3000;
const PORT_MAX = process.env.PORT_MAX ? Number(process.env.PORT_MAX) : PORT_MIN + 600;

class Portly {

	constructor(appName, portEnvName, templatePath, domain, placeholders) {

		this.appName = appName;
		this.portEnvName = portEnvName;
		this.templatePath = templatePath;
		this.domain = domain;
		this.placeholders = placeholders;

		this.content = null;
		this.__dirname = path.dirname(new URL(import.meta.url).pathname);

	};

	isPortInUse(port) {

		return new Promise((resolve) => {

			const server = net.createServer();
			server.once("error", resolve(true));
			server.once("listening", () => {
				server.close(() => resolve(false));
			});
			server.listen(port);

		});

	};

	async getPreviousAssignedPort() {

		try {

			const envFilePath = path.join(this.__dirname, ".portly.env");
			if (!fs.existsSync(envFilePath)) return null;

			const data = await fs.promises.readFile(envFilePath, "utf-8");
			const match = data.match(new RegExp(`export ${this.portEnvName}=(\\d+)`));
			return match ? Number(match[1]) : null;

		} catch (err) {
			return null;
		};

	};

	async checkProcess(port, appName) {

		return new Promise((resolve) => {

			exec(`lsof -i :${port} -t`, (errPort, stdoutPort, stderrPort) => {

				if (errPort) {
					return resolve(false);
				};

				const portProcessId = stdoutPort.trim();

				exec(`pm2 pid ${appName}`, (errorPm2, stdoutPm2, stderrPm2) => {

					if (errorPm2) {
						return resolve(false);
					};

					const pm2ProcessId = stdoutPm2.trim();

					if (portProcessId === pm2ProcessId) {
						resolve(true);
					} else {
						resolve(false);
					};

				});

			});

		});


	};

	async getConfiguration() {

		let selectedPort;

		const previousAssignedPort = await this.getPreviousAssignedPort();
		if (previousAssignedPort && previousAssignedPort >= PORT_MIN && previousAssignedPort <= PORT_MAX) {

			const inUse = await this.isPortInUse(previousAssignedPort);
			const isSameProcess = await this.checkProcess(previousAssignedPort, this.appName);

			if (!inUse && !isSameProcess) {
				selectedPort = previousAssignedPort
			};

		};

		if (!selectedPort) {

			portfinder.setBasePort(PORT_MIN);
			portfinder.setHighestPort(PORT_MAX);

			try {

				selectedPort = await portfinder.getPortPromise();
				console.log("Available port found:", selectedPort);

			} catch (err) {
				throw new Error(`Error finding available port: ${err}`);
			};

		}


		process.env[this.portEnvName] = selectedPort;

		const envFilePath = path.join(this.__dirname, ".portly.env");
		await fs.promises.writeFile(envFilePath, `export ${this.portEnvName}=${selectedPort}\n`, 'utf-8');

		try {

			const templateFullPath = path.join(this.__dirname, this.templatePath);
			const data = await fs.promises.readFile(templateFullPath, "utf-8");

			let content = data;

			this.placeholders.forEach(([key, value]) => {
				content = content.replaceAll(`{{${key}}}`, key === "PORT" ? selectedPort : value);
			});

			this.content = content;
			return this;

		} catch (err) {

			throw new Error(`Error reading template file: ${err}`);

		};

	};

	async writeConfigFile() {

		if (!this.content) return null;

		const configDir = path.join(this.__dirname, "./nginx-configs");
		const nginxConfigFilePath = path.join(configDir, this.domain);

		try {

			await fs.promises.mkdir(configDir, { recursive: true });
			await fs.promises.writeFile(nginxConfigFilePath, this.content, { encoding: "utf-8" });

			console.log(`Config written to ${nginxConfigFilePath}`);

		} catch (err) {

			throw new Error(`Cannot write Nginx config: ${err}`);

		};

	};
};

(async () => {

	const appName = process.env.APP_NAME;
	const domain = process.env.DOMAIN;
	const envName = process.env.PORT_ENV_NAME || "PORT";

	if (!appName) {
		console.error(`Portly configuration error: Missing required environment variable APP_NAME.`);
		process.exit(1);
	};

	if (!domain) {
		console.error(`Portly configuration error: Missing required environment variable DOMAIN.`);
		process.exit(1);
	};

	try {

		const portly = new Portly(
			appName,
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
	} catch (err) {

		console.error(`Portly execution failed: ${err}`);
		process.exit(1);

	};

})();