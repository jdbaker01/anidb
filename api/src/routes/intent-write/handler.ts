import { FastifyInstance } from "fastify";

export async function intentWriteRoutes(server: FastifyInstance) {
  server.post("/write", async (_request, reply) => {
    return reply.status(501).send({
      error: "Not yet implemented",
      message: "Intent write endpoint — Phase 6",
    });
  });
}
