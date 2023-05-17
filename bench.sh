#/bin/zsh

cargo build --profile=release-with-debug
# /usr/bin/time -h -p leaks -atExit -- target/release-with-debug/proj $@
/usr/bin/time -h -p -l target/release-with-debug/proj $@ >/dev/null