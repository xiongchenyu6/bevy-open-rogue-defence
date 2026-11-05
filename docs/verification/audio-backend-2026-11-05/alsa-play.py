#!/usr/bin/env python3
"""Actual libasound default PCM playback using only caller's private configuration."""
import ctypes as c
import sys, wave
alsa=c.CDLL('/nix/store/y4iw2bnyk2l5xgx5ayhj8sp01928sh3x-alsa-lib-1.2.16.1/lib/libasound.so.2')
pcm=c.c_void_p()
alsa.snd_pcm_open.argtypes=[c.POINTER(c.c_void_p),c.c_char_p,c.c_int,c.c_int]
alsa.snd_pcm_set_params.argtypes=[c.c_void_p,c.c_int,c.c_int,c.c_uint,c.c_uint,c.c_int,c.c_uint]
alsa.snd_pcm_writei.argtypes=[c.c_void_p,c.c_void_p,c.c_ulong]
alsa.snd_pcm_writei.restype=c.c_long
alsa.snd_pcm_drain.argtypes=[c.c_void_p]
alsa.snd_pcm_close.argtypes=[c.c_void_p]
assert alsa.snd_pcm_open(c.byref(pcm),b'default',0,0)==0
try:
    with wave.open(sys.argv[1]) as wav:
        assert wav.getsampwidth()==2
        channels=wav.getnchannels()
        assert alsa.snd_pcm_set_params(pcm,2,3,channels,wav.getframerate(),1,100000)==0
        frames=0
        while data:=wav.readframes(1024):
            buffer=c.create_string_buffer(data)
            count=len(data)//(2*channels)
            written=alsa.snd_pcm_writei(pcm,buffer,count)
            assert written==count, (written,count)
            frames+=written
        assert alsa.snd_pcm_drain(pcm)==0
        print(f'ALSA_PRIVATE_PLAY_PASS frames={frames} channels={channels}')
finally:
    alsa.snd_pcm_close(pcm)
