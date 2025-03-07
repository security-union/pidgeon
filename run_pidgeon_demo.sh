#!/bin/bash
set -e  # Exit on error

# Create a function to clean up processes on exit
cleanup() {
    echo "Cleaning up..."
    
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
"

echo "Starting Pidgeon PID Controller Demo..."
echo "This script will start all components needed for the demo:"
echo " - Pidgeoneer Leptos web server (port 3000)"
echo " - A demonstration PID controller"
echo ""
echo "Press Ctrl+C to stop all components"
echo "------------------------------------------------------"

# Step 1: Start the Pidgeoneer web server
echo "Starting Pidgeoneer web server on port 3000..."
cd crates/pidgeoneer
cargo run --bin server &
SERVER_PID=$!
cd ../..
echo "Pidgeoneer web server started with PID: $SERVER_PID"
echo ""

# Give server time to start
echo "Waiting for server to initialize..."
sleep 3

# Step 2: Run the temperature control example
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