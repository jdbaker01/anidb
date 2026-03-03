import { FastifyInstance } from "fastify";

export async function intentReadRoutes(server: FastifyInstance) {
  server.post("/read", async (_request, reply) => {
    return reply.status(501).send({
      error: "Not yet implemented",
      message: "Intent read endpoint — Phase 6",
    });
  });
}
