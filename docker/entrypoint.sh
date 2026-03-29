#!/bin/sh
set -e

echo "Starting Creative AI Studio service..."

# Wait briefly for dependent services to become ready
sleep 2

echo "Running service: $@"
exec "$@"
