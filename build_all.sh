#!/bin/bash
set +eoux pipefail

ALL_FLAGS="$*"

if [ -d "deployment" ]; then
  cd deployment
fi

for file in */docker-compose.yml; do
  if [ -f "$file" ]; then
    DIR=$(dirname "$file")
    pushd "$DIR"
    echo "$(date "+%Y-%m-%d %H:%M:%S") Building $DIR"
    docker compose build $ALL_FLAGS
    popd
  fi
done
