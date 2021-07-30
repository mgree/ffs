#!/bin/sh

[ -d micro ] && rm -r micro
mkdir micro
for size in 1 2 4 8 16 32 64 128 256
do
    for approach in deep wide
    do
#        if [ "$approach" = "deep" ] && [ "$size" -ge 128 ]
#        then
#            continue
#        fi
        
        for kind in list named
        do
            file="micro/${kind}_${approach}_${size}.json"
            ../utils/synth_json $kind $approach $size >$file 2>/dev/null
            if [ $? -ne 0 ] || ! [ -s $file ]
            then
                echo "Couldn't build $file."
                rm $file
            fi
        done
    done
done
