import portfinder from "portfinder";
import fs from "fs";
import path from "path";

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

		console.log("Available port found:", availablePort);

		process.env[this.portEnvName] = availablePort;
		
		const envFilePath = path.join(this.__dirname, ".portly.env");
		await fs.promises.writeFile(envFilePath, `export ${this.portEnvName}=${availablePort}\n`, 'utf-8');

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

	const domain = process.env.DOMAIN;
	const envName = process.env.PORT_ENV_NAME || "PORT";

	if (!domain) {
		console.error(`Portly configuration error: Missing required environment variable DOMAIN.`);
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
	} catch (err) {

		console.error(`Portly execution failed: ${err}`);
		process.exit(1);

	};

})();