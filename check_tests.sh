#!/bin/bash
export LD_LIBRARY_PATH=/home/user/roup/compat/ompparser/build:/home/user/roup/target/release:$LD_LIBRARY_PATH
cd /home/user/roup/compat/ompparser/ompparser/build-roup

for f in ../tests/*.txt; do
    name=$(basename "$f" .txt)
    result=$(./tester-roup "$f" 2>&1 | grep "PASSED TESTS\|FAILED TESTS")
    passed=$(echo "$result" | grep "PASSED" | awk '{print $4}')
    failed=$(echo "$result" | grep "FAILED" | awk '{print $4}')
    passed=${passed:-0}
    failed=${failed:-0}
    total=$((passed + failed))
    if [ "$total" -gt 0 ]; then
        rate=$(echo "scale=1; $passed*100/$total" | bc)
        echo "$rate% ($passed/$total) - $name"
    fi
done | sort -n
