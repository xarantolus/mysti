#!/bin/bash
set -eoux pipefail

ALL_FLAGS="$*"

if [ -d "deployment" ]; then
  cd deployment
fi

for file in */docker-compose.yml; do
  if [ -f "$file" ]; then
    DIR=$(dirname "$file")
    pushd "$DIR"
    echo "$(date "+%Y-%m-%d %H:%M:%S") Building $DIR"

    # Run in background - that way we can build multiple images at once
    docker compose build $ALL_FLAGS &
    popd
  fi
done

# Wait for all background processes to finish
wait
