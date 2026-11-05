#!/bin/sh
. /home/freeman.xiong/Documents/github/xiongchenyu6/bevy-open-rogue-defence/target/verification/audio-backend-2026-11-05/env.sh
exec /nix/store/sn55x89zpz63bjpajwp706hab6k5b0z5-pulseaudio-17.0/bin/pulseaudio -n --use-pid-file=no --daemonize=no --exit-idle-time=-1 --disable-shm=yes --file=/home/freeman.xiong/Documents/github/xiongchenyu6/bevy-open-rogue-defence/target/verification/audio-backend-2026-11-05/private.pa
