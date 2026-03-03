import Fastify from "fastify";
import cors from "@fastify/cors";
import sensible from "@fastify/sensible";
import { config } from "dotenv";

config();

const server = Fastify({
  logger: {
    level: process.env.LOG_LEVEL || "info",
  },
});

await server.register(cors);
await server.register(sensible);

server.get("/health", async () => ({ status: "ok" }));

// Route placeholders — implemented in Phase 6
// server.register(intentReadRoutes, { prefix: '/api/v1/intent' });
// server.register(intentWriteRoutes, { prefix: '/api/v1/write' });
// server.register(subscriptionRoutes, { prefix: '/api/v1/subscriptions' });

const PORT = parseInt(process.env.API_PORT || "3000", 10);

try {
  await server.listen({ port: PORT, host: "0.0.0.0" });
} catch (err) {
  server.log.error(err);
  process.exit(1);
}
