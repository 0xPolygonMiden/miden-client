find . -type f -name "wait_time_*" -delete

LOG_WAIT_TIMES=true make integration-test

echo "Number of waits: $(find . -type f -name "wait_time_*" | wc -l)"
milliseconds_waited=$(find . -name "wait_time_*" -type f | xargs cat | awk '{sum += $1} END {print sum}')
seconds_waited=$(awk "BEGIN {printf \"%.2f\", $milliseconds_waited/1000}")
echo "Seconds waited: $seconds_waited"
find . -type f -name "wait_time_*" -delete
