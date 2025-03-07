#!/bin/bash
set -e  # Exit on error

# Create a function to clean up processes on exit
cleanup() {
    echo "Cleaning up..."
    
    # Kill background processes
    if [ -n "$IGGY_PID" ]; then
        echo "Stopping iggy server..."
        docker stop iggy_server >/dev/null 2>&1 || true
    fi
    
    if [ -n "$SERVER_PID" ]; then
        echo "Stopping pidgeoneer server..."
        kill -9 $SERVER_PID >/dev/null 2>&1 || true
    fi
    
    if [ -n "$CONTROLLER_PID" ]; then
        echo "Stopping PID controller..."
        kill -9 $CONTROLLER_PID >/dev/null 2>&1 || true
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
                                                       
    ____  __                                               
   / __ \/  |_____________  ____  ____  ___  ___  _____   
  / /_/ />  </ __/ __/ __ \/ __ \/ __ \/ _ \/ _ \/ ___/   
 / ____/>  </ /_/ /_/ /_/ / / / / / / /  __/  __/ /       
/_/    /_/|_\__/\__/\____/_/ /_/_/ /_/\___/\___/_/        
                                                           
"

echo "Starting Pidgeon PID Controller Demo..."
echo "This script will start all components needed for the demo:"
echo " - iggy server (Docker container)"
echo " - Pidgeoneer Leptos web application"
echo " - PID controller example with debugging"
echo ""
echo "Press Ctrl+C to stop all components"
echo "------------------------------------------------------"

# Step 1: Start iggy server in Docker
echo "Starting iggy server in Docker..."
if ! docker ps | grep -q iggy_server; then
    docker run --rm -d -p 8090:8090 --name iggy_server iggyrs/iggy
    IGGY_PID=1  # Just a marker that Docker is running
    echo "Iggy server started successfully!"
else
    echo "Iggy server is already running"
    IGGY_PID=1
fi
echo ""

# Give iggy server time to start
echo "Waiting for iggy server to initialize..."
sleep 3

# Step 2: Start the Leptos server
echo "Starting Pidgeoneer web application..."
cd crates/pidgeoneer
cargo leptos watch &
SERVER_PID=$!
cd ../..
echo "Pidgeoneer server started with PID: $SERVER_PID"
echo ""

# Give server time to start
echo "Waiting for server to initialize..."
sleep 5

# Step 3: Run a simple temperature control example
echo "Starting PID controller example..."
cargo run --package pidgeon --example temperature_control &
CONTROLLER_PID=$!
echo "PID controller example started with PID: $CONTROLLER_PID"
echo ""

echo "All components have been started!"
echo "Open your browser to http://localhost:3000 to view the dashboard"
echo ""
echo "Press Ctrl+C to stop all components"

# Wait for user to press Ctrl+C
wait $CONTROLLER_PID 