#!/bin/sh
docker run --rm --user "$(id -u)":"$(id -g)" -v "$PWD":/tmp/build -v ~/.cargo/:"/home/$(whoami)/.cargo/" -w /tmp/build registry.undertheprinter.com/rust-arm-cross:latest cargo build --target aarch64-unknown-linux-gnu
docker build -t registry.undertheprinter.com/stomata:latest .
docker push registry.undertheprinter.com/stomata:latest
kubectl rollout restart deployment -n thyme stomata
