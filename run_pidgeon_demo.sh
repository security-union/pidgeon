#!/bin/bash
set -e  # Exit on error

IGGY_CONTAINER_NAME="pidgeon-iggy"
# Pin to 0.4.203 — matches iggy crate 0.6.203 used by pidgeon/pidgeoneer.
# Do NOT use 'latest' — iggy frequently ships breaking changes.
IGGY_IMAGE="iggyrs/iggy:0.4.203"

# Create a function to clean up processes on exit
cleanup() {
    echo ""
    echo "Cleaning up..."

    if [ -n "$CONTROLLER_PID" ]; then
        echo "Stopping PID controller..."
        kill $CONTROLLER_PID >/dev/null 2>&1 || true
    fi

    if [ -n "$SERVER_PID" ]; then
        echo "Stopping pidgeoneer server..."
        kill $SERVER_PID >/dev/null 2>&1 || true
    fi

    if [ "$STARTED_IGGY" = true ]; then
        echo "Stopping Iggy server..."
        docker stop $IGGY_CONTAINER_NAME >/dev/null 2>&1 || true
    fi

    echo "Cleanup complete!"
}

# Register the cleanup function to run on exit
trap cleanup EXIT

# Print banner
echo "
    ____  _     __
   / __ \(_)___/ /_____ ____  ____  ___  ___  _____
  / /_/ / / __  / / __ \/ __ \/ __ \/ _ \/ _ \/ ___/
 / ____/ / /_/ / / /_/ / / / / /_/ /  __/  __/ /
/_/   /_/\__,_/_/\__, /_/ /_/\____/\___/\___/_/
                /____/
"

echo "Starting Pidgeon PID Controller Demo..."
echo "This script will start all components needed for the demo:"
echo " 1. Iggy message server (port 8090)"
echo " 2. Pidgeoneer Leptos web server (port 3000)"
echo " 3. A demonstration PID controller"
echo ""
echo "Press Ctrl+C to stop all components"
echo "------------------------------------------------------"

# ── Step 1: Start the Iggy message server ──────────────────────────────

STARTED_IGGY=false

# Check if Iggy is already running on port 8090
if curl -s --connect-timeout 2 http://localhost:8090 >/dev/null 2>&1 || \
   nc -z localhost 8090 >/dev/null 2>&1; then
    echo "Iggy server already running on port 8090, skipping startup."
else
    echo "Starting Iggy message server on port 8090..."

    if ! command -v docker >/dev/null 2>&1; then
        echo "Error: Docker is required to run the Iggy message server."
        echo "Install Docker from https://docs.docker.com/get-docker/"
        echo "Or start an Iggy server manually on port 8090 before running this script."
        exit 1
    fi

    # Remove any leftover container from a previous run
    docker rm -f $IGGY_CONTAINER_NAME >/dev/null 2>&1 || true

    docker run -d --rm \
        --name $IGGY_CONTAINER_NAME \
        -p 8090:8090 \
        $IGGY_IMAGE >/dev/null 2>&1

    STARTED_IGGY=true

    # Wait for Iggy to be ready
    echo -n "Waiting for Iggy server"
    MAX_WAIT=30
    WAITED=0
    IGGY_READY=false

    while [ $WAITED -lt $MAX_WAIT ]; do
        if nc -z localhost 8090 >/dev/null 2>&1; then
            IGGY_READY=true
            break
        fi
        printf "."
        sleep 1
        WAITED=$((WAITED + 1))
    done

    echo ""

    if [ "$IGGY_READY" = false ]; then
        echo "Error: Iggy server did not start within ${MAX_WAIT}s."
        echo "Check Docker logs: docker logs $IGGY_CONTAINER_NAME"
        exit 1
    fi

    echo "Iggy server is ready."
fi

echo ""

# ── Step 2: Start the Pidgeoneer web server ────────────────────────────

echo "Starting Pidgeoneer web server on port 3000..."
cd crates/pidgeoneer

# Check if cargo-leptos is installed
if ! cargo leptos --version >/dev/null 2>&1; then
    echo "Error: cargo-leptos is not installed."
    echo "Please install it with: cargo install cargo-leptos"
    exit 1
fi

# Start the server in the background
nohup cargo leptos watch > leptos_server.log 2>&1 &
SERVER_PID=$!

if ! ps -p $SERVER_PID > /dev/null; then
    echo "Error: Failed to start the server. Please check for errors."
    exit 1
fi

cd ../..
echo "Pidgeoneer web server started with PID: $SERVER_PID"
echo ""

# Wait for the server to actually be ready (compilation + startup)
echo -n "Waiting for server to compile and start (this may take a minute)"
MAX_WAIT=120
WAITED=0
SERVER_READY=false

while [ $WAITED -lt $MAX_WAIT ]; do
    if ! ps -p $SERVER_PID > /dev/null; then
        echo ""
        echo "Error: Server process died during compilation."
        echo "Check crates/pidgeoneer/leptos_server.log for details."
        exit 1
    fi

    if curl -s --head --fail http://localhost:3000 >/dev/null 2>&1; then
        SERVER_READY=true
        break
    fi

    printf "."
    sleep 2
    WAITED=$((WAITED + 2))
done

echo ""

if [ "$SERVER_READY" = false ]; then
    echo "Error: Server did not respond within ${MAX_WAIT}s."
    echo "Check crates/pidgeoneer/leptos_server.log for details."
    exit 1
fi

echo "Pidgeoneer server is ready."
echo ""

# ── Step 3: Open browser and start controller ──────────────────────────

# Open the browser to the dashboard
echo "Opening dashboard in your browser..."
if [[ "$OSTYPE" == "darwin"* ]]; then
    open "http://localhost:3000"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    xdg-open "http://localhost:3000" &>/dev/null || true
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    start "http://localhost:3000" || true
else
    echo "Could not automatically open browser. Please navigate to http://localhost:3000"
fi

echo ""
echo "Dashboard is ready at http://localhost:3000"
echo -n "Press 'S' to start the PID controller example: "
while true; do
    read -n 1 key
    if [[ $key == "s" || $key == "S" ]]; then
        echo ""
        break
    fi
done

# Start the temperature control example
echo "Starting PID controller example..."
cargo run --package pidgeon --example debug_temperature_control --features=debugging &
CONTROLLER_PID=$!
echo "PID controller example started with PID: $CONTROLLER_PID"
echo ""

echo "All components are running!"
echo " - Iggy server:  localhost:8090"
echo " - Dashboard:    http://localhost:3000"
echo " - Controller:   PID $CONTROLLER_PID"
echo ""
echo "Press Ctrl+C to stop all components"

# Wait for processes — keeps the script alive until Ctrl+C or process exit
wait
