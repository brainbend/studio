#!/bin/bash
# Run Studio with a memory cap: if it grows past the limit (default 4 GB) this takes a 3-second profile of what it is
# doing, saves it, and kills it, so a runaway cannot take the whole computer down.
#   scripts/run-capped.sh                      run target/release/brainbend-studio, 4 GB cap
#   scripts/run-capped.sh path/to/binary 2048  another binary, 2 GB cap
BIN=${1:-target/release/brainbend-studio}
LIMIT_MB=${2:-4096}
"$BIN" &
PID=$!
echo "Studio is running (pid $PID); it will be stopped if it passes ${LIMIT_MB} MB."
while kill -0 $PID 2>/dev/null; do
  RSS_KB=$(ps -o rss= -p $PID | tr -d ' ')
  if [ -n "$RSS_KB" ] && [ "$RSS_KB" -gt $((LIMIT_MB * 1024)) ]; then
    OUT="$HOME/Desktop/studio-runaway-$(date +%H%M%S).txt"
    echo "Studio passed ${LIMIT_MB} MB. Saving a profile to $OUT and stopping it."
    sample $PID 3 -file "$OUT" > /dev/null 2>&1
    kill -9 $PID
    break
  fi
  sleep 1
done
