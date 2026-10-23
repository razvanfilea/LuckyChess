#!/usr/bin/env bash
# Runs the engine bench for every build inside the universal binary, interleaved, and prints medians.
#
# Usage: bench_universal.sh [path to universal binary] [runs]

BIN=${1:-./lucky_chess}
RUNS=${2:-15}
ARCHES="auto x86_64_v2 x86_64_v3_nopext x86_64_v3 x86_64_v4"

echo "CPU: $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2-)"
echo "AVX-512 flags: $(grep -m1 flags /proc/cpuinfo | grep -o 'avx512[a-z_0-9]*' | sort -u | tr '\n' ' ')"
echo

run() {
    LUCKY_CHESS_ARCH=$1 "$BIN" bench
}

# Sanity check: every build must search the same number of nodes
for a in $ARCHES; do
    echo "$a: $(run "$a" | awk '/Nodes searched/{print $4}') nodes"
done
echo

for i in $(seq "$RUNS"); do
    echo -ne "\rRun $i/$RUNS" >&2
    for a in $ARCHES; do
        echo "$a $(run "$a" | awk '/Nodes\/second/{print $3}')"
    done
done | sort -k1,1 -k2n | awk -v arches="$ARCHES" '
    { v[$1] = v[$1] " " $2 }
    END {
        printf "\n%-18s %10s %10s %10s\n", "build", "median", "min", "max"
        split(arches, order, " ")
        for (i = 1; i in order; i++) {
            k = order[i]
            n = split(v[k], x, " ")
            printf "%-18s %10d %10d %10d\n", k, x[int((n + 1) / 2)], x[1], x[n]
        }
    }'
