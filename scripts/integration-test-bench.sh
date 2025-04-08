# Using the LOG_WAIT_TIMES env var we can configure `wait_for_tx` to store each wait time in a file named `wait_time_<uuid>`.
# This script runs the integration tests with the LOG_WAIT_TIMES env var set to true and then sums all the wait times to get
# the total time waited for all the tests.
# The script can be used to benchmark the performance of the integration tests in terms of time waited for transactions to be
# included in the blockchain.

find . -type f -name "wait_time_*" -delete

LOG_WAIT_TIMES=true make integration-test

echo "Number of waits: $(find . -type f -name "wait_time_*" | wc -l)"
milliseconds_waited=$(find . -name "wait_time_*" -type f | xargs cat | awk '{sum += $1} END {print sum}')
seconds_waited=$(awk "BEGIN {printf \"%.2f\", $milliseconds_waited/1000}")
echo "Seconds waited: $seconds_waited"
find . -type f -name "wait_time_*" -delete
