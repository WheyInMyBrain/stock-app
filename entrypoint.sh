#!/bin/sh
set -e

# Case 1: If the user passed explicit CLI flags, bypass the daemon loop and execute directly
if [ "$#" -gt 0 ] && [ "$1" != "daemon" ]; then
    exec "$@"
    exit 0
fi

echo "================================================================================="
echo "🪐 DATA ENGINE INFRASTRUCTURE DAEMON ACTIVE & RUNNING IN PERSISTENT BACKGROUND"
echo "================================================================================="
echo "🚀 Both Go Downloader and Rust Parser are fully compiled and ready."
echo "👉 You can execute jobs inside this container anytime via docker exec."
echo "================================================================================="

# Case 2: Standard Daemon Loop Mode. Keeps the container alive indefinitely.
while true; do
    sleep 3600
done