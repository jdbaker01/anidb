#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# ANIDB — full local stack launcher
#
# Usage:  ./dev.sh
#
# What it does:
#   1. Starts Docker infrastructure (Neo4j, EventStoreDB, Postgres, Redis)
#   2. Waits for all health checks to pass
#   3. Builds all Rust services
#   4. Opens a tmux session with one window per service:
#        event-log | confidence | ontology | subscript | semantic | api
#
# Requirements: docker, tmux, cargo, node/npm
#   Install tmux:  brew install tmux
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CORE="$REPO/core"
API="$REPO/api"
export PATH="/Users/bakerj/.cargo/bin:$PATH"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

die()  { echo -e "${RED}ERROR: $*${NC}" >&2; exit 1; }
info() { echo -e "${CYAN}▶ $*${NC}"; }
ok()   { echo -e "  ${GREEN}✓${NC} $*"; }

# ── Preflight ─────────────────────────────────────────────────────────────────
command -v docker  >/dev/null || die "docker not found"
command -v tmux    >/dev/null || die "tmux not found — install with: brew install tmux"
command -v cargo   >/dev/null || die "cargo not found — install from https://rustup.rs"
command -v node    >/dev/null || die "node not found — install from https://nodejs.org"
[ -f "$REPO/.env" ] || die ".env not found — run: cp .env.example .env"

# ── 1. Docker infrastructure ──────────────────────────────────────────────────
info "Starting Docker infrastructure..."
docker compose -f "$REPO/docker-compose.yml" up -d \
    || die "docker compose failed — is Docker running?"

info "Waiting for infrastructure health checks..."
INFRA_SERVICES=(anidb-neo4j anidb-eventstore anidb-postgres anidb-redis)
MAX_WAIT=120  # seconds

for svc in "${INFRA_SERVICES[@]}"; do
    elapsed=0
    echo -n "  waiting for $svc..."
    until [ "$(docker inspect --format='{{.State.Health.Status}}' "$svc" 2>/dev/null)" = "healthy" ]; do
        if [ "$elapsed" -ge "$MAX_WAIT" ]; then
            echo ""
            die "$svc did not become healthy within ${MAX_WAIT}s"
        fi
        echo -n "."
        sleep 3
        elapsed=$((elapsed + 3))
    done
    echo -e " ${GREEN}healthy${NC}"
done

# ── 2. Build Rust services ────────────────────────────────────────────────────
info "Building Rust services (this may take a minute on first run)..."
(cd "$CORE" && cargo build 2>&1) || die "cargo build failed"
ok "Rust build complete"

# ── 3. Ensure api/.env exists ────────────────────────────────────────────────
if [ ! -f "$API/.env" ] && [ ! -L "$API/.env" ]; then
    ln -s "$REPO/.env" "$API/.env"
    ok "Created api/.env symlink"
fi

# ── 4. Install API deps if needed ────────────────────────────────────────────
if [ ! -d "$API/node_modules" ]; then
    info "Installing API gateway dependencies..."
    (cd "$API" && npm install) || die "npm install failed"
    ok "npm install complete"
fi

# ── 5. Launch tmux session ────────────────────────────────────────────────────
SESSION="anidb"
tmux kill-session -t "$SESSION" 2>/dev/null || true

# All panes cd to $CORE so dotenvy finds core/.env -> ../.env
CARGO="cd '$CORE' && cargo run -p"

tmux new-session  -d -s "$SESSION" -n "event-log"   -x 220 -y 50
tmux send-keys    -t "$SESSION:=event-log"   "$CARGO anidb-event-log" Enter

tmux new-window   -t "$SESSION" -n "confidence"
tmux send-keys    -t "$SESSION:=confidence"  "$CARGO anidb-confidence-store" Enter

tmux new-window   -t "$SESSION" -n "ontology"
tmux send-keys    -t "$SESSION:=ontology"    "$CARGO anidb-ontology" Enter

tmux new-window   -t "$SESSION" -n "subscript"
tmux send-keys    -t "$SESSION:=subscript"   "$CARGO anidb-subscription-engine" Enter

tmux new-window   -t "$SESSION" -n "semantic"
tmux send-keys    -t "$SESSION:=semantic"    "$CARGO anidb-semantic-engine" Enter

tmux new-window   -t "$SESSION" -n "api"
tmux send-keys    -t "$SESSION:=api"         "cd '$API' && npm run dev" Enter

tmux select-window -t "$SESSION:=event-log"

# ── 6. Print cheatsheet then attach ──────────────────────────────────────────
echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}  ANIDB stack is starting                           ${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  ${YELLOW}tmux windows${NC} (Ctrl+b then number):"
echo "    1  event-log          :8010"
echo "    2  confidence-store   :8003"
echo "    3  ontology           :8002"
echo "    4  subscription-engine"
echo "    5  semantic-engine    :8001"
echo "    6  api-gateway        :3000"
echo ""
echo -e "  ${YELLOW}tmux shortcuts${NC}:"
echo "    Ctrl+b n/p   next/previous window"
echo "    Ctrl+b d     detach (services keep running)"
echo "    Ctrl+b &     kill current window"
echo ""
echo -e "  ${YELLOW}Reattach later${NC}:  tmux attach -t anidb"
echo -e "  ${YELLOW}Stop everything${NC}: tmux kill-session -t anidb && docker compose down"
echo ""

tmux attach-session -t "$SESSION"
