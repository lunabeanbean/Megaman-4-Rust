#!/bin/sh
# Records the weapon regression set (seeds 91xxx, KEEP=1). Takes about 20 minutes.
for w in 4 6 7 8 9 10 11 12 13; do
  b=$((91000 + w * 10))
  sh wtest.sh $w 9C 900 $((b+1)) $((b+2)) $((b+3)) 2>&1 | cut -c1-110
done
# enemies can hurt (death stops the comparison)
for w in 9 11 12; do
  b=$((91500 + w * 10))
  NOINV=1 sh wtest.sh $w 9C 1500 $((b+1)) $((b+2)) $((b+3)) 2>&1 | cut -c1-110
done
# low energy (3 units) so the weapon runs dry
for w in 9 13; do
  b=$((91800 + w * 10))
  sh wtest.sh $w 83 900 $((b+1)) $((b+2)) 2>&1 | cut -c1-110
done
