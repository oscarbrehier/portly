import portfinder from "portfinder";
import fs from "fs";
import path from "path";

const PORT_MIN = process.env.PORT_MIN ? Number(process.env.PORT_MIN) : 3000;
const PORT_MAX = process.env.PORT_MAX ? Number(process.env.PORT_MAX) : PORT_MIN + 600;

async function getConfiguration(filepath, placeholders) {

	let availablePort;

	portfinder.setBasePort(PORT_MIN);
	portfinder.setHighestPort(PORT_MAX);

	try {
		availablePort = await portfinder.getPortPromise();
	} catch (err) {
		console.error("Error finding available port:", err);
		return null;
	}

	if (!availablePort) return null;

	try {

		const __dirname = path.dirname(new URL(import.meta.url).pathname);
		const templatePath = path.join(__dirname, filepath);

		const data = await fs.promises.readFile(templatePath, "utf-8");

		let content = data;
		placeholders.forEach(([key, value]) => {
			content = content.replaceAll(`{{${key}}}`, key === "PORT" ? availablePort : value);
		});

		return content;
	} catch (err) {
		console.error("Error reading template file:", err);
		return null;
	}
}

const config = await getConfiguration(
	"./nginx_template.txt",
	[
		["DOMAIN", "chat.eggspank.cloud"],
		["PORT", ""]
	]
);

console.log(config);
