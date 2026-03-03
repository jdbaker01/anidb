import { FastifyInstance } from "fastify";

export async function subscriptionRoutes(server: FastifyInstance) {
  server.post("/register", async (_request, reply) => {
    return reply.status(501).send({
      error: "Not yet implemented",
      message: "Subscription registration — Phase 6",
    });
  });
}
