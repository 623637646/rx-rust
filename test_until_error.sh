#!/bin/bash

trap "echo 'Interrupted by user'; exit 130" SIGINT

count=0

while true; do
  ((count++))
  echo "===== Loop #$count started ====="

  start_time=$(date +%s)
  tmpfile=$(mktemp)

  RUST_BACKTRACE=1 cargo hack --each-feature --exclude-all-features --exclude-no-default-features --exclude-features default,multi-threaded,single-threaded nextest run --no-fail-fast --failure-output final --status-level=fail 2>&1 | tee "$tmpfile"

  end_time=$(date +%s)
  elapsed=$((end_time - start_time))

  echo "===== Loop #$count finished in ${elapsed}s ====="

  if grep -q "error" "$tmpfile"; then
    mkdir -p logs
    timestamp=$(date +"%Y%m%d_%H%M%S")
    logfile="logs/test_${timestamp}_${count}.log"
    mv "$tmpfile" "$logfile"
    echo "Error detected. Output saved to $logfile. Exiting loop."
    break
  else
    rm "$tmpfile"
  fi
done
